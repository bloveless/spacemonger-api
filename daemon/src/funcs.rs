use crate::db;
use spacetraders::client::{Client, ArcHttpClient};
use spacetraders::{shared, responses, client};
use tokio::time::Duration;
use std::convert::TryFrom;
use sqlx::PgPool;

#[derive(Debug)]
pub(crate) struct User {
    pub(crate) username: String,
    assignment: String,
    pub(crate) system_symbol: Option<String>,
    pub(crate) location_symbol: Option<String>,
    pub(crate) client: Client,
}

pub(crate) async fn get_user(http_client: ArcHttpClient, pg_pool: PgPool, username: String, assignment: String, system_symbol: Option<String>, location_symbol: Option<String>) -> Result<User, Box<dyn std::error::Error>> {
    let db_user = db::get_user(pg_pool.clone(), username.to_owned()).await?;

    if let Some(user) = db_user {
        println!("Found existing user {}", username);
        Ok(
            User {
                username,
                assignment,
                system_symbol,
                location_symbol,
                client: Client::new(http_client, user.id, user.username, user.token),
            }
        )
    } else {
        println!("Creating new user {}", username);
        let claimed_user = client::claim_username(http_client.clone(), username.to_owned()).await?;

        println!("Claimed new user {:?}", claimed_user);

        let user = db::persist_user(
            pg_pool.clone(),
            username.to_owned(),
            claimed_user.token.to_owned(),
            assignment.to_owned(),
            system_symbol.to_owned(),
            location_symbol.to_owned(),
        ).await?;

        println!("New user persisted");

        Ok(
            User {
                username: username.to_owned(),
                assignment,
                system_symbol,
                location_symbol,
                client: Client::new(http_client.clone(), user.id, username.to_owned(), claimed_user.token.to_owned()),
            }
        )
    }
}


pub async fn create_flight_plan(client: &Client, pg_pool: PgPool, ship: &shared::Ship, destination: String) -> Result<responses::FlightPlan, Box<dyn std::error::Error>> {
    let flight_plan = client.create_flight_plan(ship.id.to_owned(), destination.to_owned()).await?;

    db::persist_flight_plan(pg_pool, client.user_id.clone(), ship, &flight_plan).await?;

    Ok(flight_plan)
}

pub async fn get_systems(client: &Client, pg_pool: PgPool) -> Result<responses::SystemsInfo, Box<dyn std::error::Error>> {
    let systems_info = client.get_systems_info().await?;
    println!("Systems info: {:?}", systems_info);

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

pub async fn scan_system(client: &Client, ship: shared::Ship, pg_pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
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
                let flight_plan = create_flight_plan(client, pg_pool.clone(), &ship, location.symbol.clone()).await?;
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
