use spacetraders::client::Client;
use spacetraders::{shared, responses};
use tokio::time::Duration;
use std::convert::TryFrom;
use crate::db;

pub async fn create_flight_plan(client: &Client, pg_pool: db::PgPool, user_id: String, ship: &shared::Ship, destination: String) -> Result<responses::FlightPlan, Box<dyn std::error::Error>> {
    let flight_plan = client.create_flight_plan(ship.id.to_owned(), destination.to_owned()).await?;

    db::persist_flight_plan(pg_pool, user_id, ship, &flight_plan).await?;

    Ok(flight_plan)
}

pub async fn get_systems(client: &Client, pg_pool: db::PgPool) -> Result<responses::SystemsInfo, Box<dyn std::error::Error>> {
    let systems_info = client.get_systems_info().await?;
    println!("Systems info: {:?}", systems_info);

    db::truncate_system_info(pg_pool.clone()).await?;

    for system in &systems_info.systems {
        for location in &system.locations {
            db::persist_system_location(pg_pool.clone(), system, location).await?;
        }
    }

    Ok(systems_info)
}

pub async fn get_fastest_ship(client: &Client) -> Result<Option<shared::Ship>, Box< dyn std::error::Error>> {
    let user_info = client.get_user_info().await?;

    let mut fastest_ship_speed = 0;
    let mut fastest_ship = None;

    for ship in user_info.user.ships {
        if ship.speed > fastest_ship_speed {
            fastest_ship = Some(ship.to_owned());
            fastest_ship_speed = ship.speed;
        }
    }

    Ok(fastest_ship)
}

pub async fn get_ship(client: &Client, ship_id: String) -> Result<Option<shared::Ship>, Box<dyn std::error::Error>> {
    let user_info = client.get_user_info().await?;

    let mut ship = None;

    for current_ship in user_info.user.ships {
        if current_ship.id == ship_id {
            ship = Some(current_ship.to_owned());
        }
    }

    Ok(ship)
}

pub async fn scan_system(client: &Client, ship: shared::Ship, pg_pool: db::PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let mut ship = ship.clone();
    let systems_info = get_systems(client, pg_pool.clone()).await?;

    // Fill the ship as full as possible with fuel
    let ship_cargo_count = ship.cargo.iter().fold(0, |sum, cargo| sum + cargo.quantity);
    if ship_cargo_count < ship.max_cargo {
        let purchase_order_response = client.create_purchase_order(
            ship.clone(),
            shared::Good::Fuel,
            ship.max_cargo - ship_cargo_count,
        ).await?;

        println!("Fill up ship ------------------------------------------------------------------");
        println!("{:?}", purchase_order_response);
    }

    // Then set a course and wait for the ship to arrive at each location

    for system in systems_info.systems {
        for location in system.locations {
            println!("Location symbol: {}", location.symbol);

            // Don't attempt to fly to a location that the ship is already at
            if ship.clone().location != Some(location.symbol.clone()) {
                let flight_plan = create_flight_plan(client, pg_pool.clone(), client.user_id.to_owned(), &ship, location.symbol.clone()).await?;
                println!("Flight plan: {:?}", &flight_plan);

                println!("Waiting for {} seconds", flight_plan.flight_plan.time_remaining_in_seconds + 5);
                tokio::time::sleep(Duration::new(u64::try_from(flight_plan.flight_plan.time_remaining_in_seconds + 5).unwrap(), 0)).await;

                ship.location = Some(location.symbol.clone());
            }

            let marketplace_info = client.get_location_marketplace(location.symbol.clone()).await?;

            for datum in marketplace_info.location.marketplace {
                println!("Location: {}, Good: {:?}, Available: {}, Price Per Unit: {}", &location.symbol, &datum.symbol, &datum.quantity_available, &datum.price_per_unit);

                db::persist_market_data(pg_pool.clone(), &location, &datum).await?;
            }

            let ship_info = client.get_your_ships().await?;
            let ship_info = ship_info.ships.iter().find(|s| s.id == ship.id).unwrap().to_owned();

            let ship_fuel = ship_info.cargo.iter().fold(0, |sum, cargo| if cargo.good == shared::Good::Fuel { sum + cargo.quantity } else { sum });
            println!("Current ship fuel: {}", ship_fuel);

            // If the ship is less than 2/3 full fill it all the way up!
            if ship_fuel < 66 {
                println!("Purchasing {} fuel", 100 - ship_fuel);
                client.create_purchase_order(ship.clone(), shared::Good::Fuel, 100 - ship_fuel).await?;
            }
        }
    }

    Ok(())
}
