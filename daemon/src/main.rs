mod funcs;
mod db;

use spacetraders::client;
use std::env;
use dotenv::dotenv;
use tokio::time::Duration;
use std::convert::{TryInto, TryFrom};
use spacetraders::shared::{LoanType, Good};
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let username_base = env::var("USERNAME_BASE").unwrap();
    let postgres_host = env::var("POSTGRES_HOST").unwrap();
    let postgres_port = env::var("POSTGRES_PORT").unwrap().parse::<i32>().unwrap();
    let postgres_username = env::var("POSTGRES_USERNAME").unwrap();
    let postgres_password = env::var("POSTGRES_PASSWORD").unwrap();
    let postgres_database = env::var("POSTGRES_DATABASE").unwrap();

    let pg_pool = db::get_db_pool(postgres_host, postgres_port, postgres_username, postgres_password, postgres_database).await?;

    db::run_migrations(pg_pool.clone()).await?;

    // Algorithm. Create the main user account (or get from db). Get the number of locations
    // in the system. Create (or get from db) X scout accounts (where X is number of locations in
    // the system). Send each scout account to the location they are assigned.

    let http_client = client::get_http_client();
    let main_user = funcs::get_user(http_client.clone(), pg_pool.clone(), format!("{}-main", username_base), "main".to_string(), None, None).await?;

    // When an API reset occurs all the scouts will being to fail making requests.
    // As soon as all the scouts fail this pod will restart. Upon restart we will get
    // the main user and attempt to get the system info. If the main user is unable to make a request
    // we can assume that the API has been reset and we need to reset ourselves.
    if main_user.client.get_user_info().await.is_err() {
        db::reset_db(pg_pool.clone()).await?;
        // Now that the tables have been we will panic so that the pod will restart and the tables will be recreated
        panic!("Unable to connect using the main user. Assuming an API reset. Backing up data and clearing the database");
    };

    let system_info = main_user.client.get_systems_info().await?;

    for system in &system_info.systems {
        for location in &system.locations {
            db::persist_system_location(pg_pool.clone(), system, location).await?;
        }
    }

    println!("## Begin System Messages ----------------------------------------------------------");
    for system in &system_info.systems {
        for location in &system.locations {
            if let Some(messages) = &location.messages {
                for message in messages {
                    println!("Location: {} Message: {}", location.symbol, message)
                }
            }
        }
    }
    println!("## End System Messages ------------------------------------------------------------");

    let mut scouts: Vec<funcs::User> = Vec::new();

    for system in &system_info.systems {
        for location in &system.locations {
            let scout_user = funcs::get_user(
                http_client.clone(),
                pg_pool.clone(),
                format!("{}-scout-{}", username_base, location.symbol),
                "scout".to_string(),
                Some(system.symbol.to_owned()),
                Some(location.symbol.to_owned()),
            ).await?;

            scouts.push(scout_user);
        }
    }

    println!("Main user info: {:?}", main_user.client.get_user_info().await?);

    let mut handles = Vec::new();

    for scout in scouts {
        let pg_pool = pg_pool.clone();
        handles.push(tokio::spawn(async move {
            let mut current_user_info = scout.client.get_user_info().await?;

            // 1. if the user doesn't have enough credits take out a startup loan
            println!("Scout {} -- user info {:?}", scout.username, current_user_info);
            if current_user_info.user.credits == 0 {
                println!("Scout {} -- Requesting new {:?} loan", scout.username, LoanType::Startup);
                // assume that if the user has 0 credits that the user needs to take out a loan
                scout.client.request_new_loan(LoanType::Startup).await?;
                current_user_info = scout.client.get_user_info().await?;
            }

            // 2. if the user doesn't have any ships then buy the fastest one that the user can afford that is in the system assigned to the scout
            if current_user_info.user.ships.is_empty() {
                let available_ships = scout.client.get_ships_for_sale().await?;
                let mut fastest_ship = None;
                let mut fastest_ship_speed = 0;
                let mut fastest_ship_location = "".to_string();
                let mut fastest_ship_price = 0;

                for available_ship in &available_ships.ships {
                    for purchase_location in &available_ship.purchase_locations {
                        if available_ship.speed > fastest_ship_speed
                            && current_user_info.user.credits > purchase_location.price
                            && purchase_location.location.contains(&scout.system_symbol.clone().unwrap())
                        {
                            fastest_ship_speed = available_ship.speed;
                            fastest_ship = Some(available_ship);
                            fastest_ship_location = purchase_location.location.to_owned();
                            fastest_ship_price = purchase_location.price;
                        }
                    }
                }

                if let Some(ship) = fastest_ship {
                    println!("Scout {} -- Buying {} for {} at location {}", scout.username, ship.ship_type.to_owned(), fastest_ship_price, fastest_ship_location);
                    scout.client.purchase_ship(fastest_ship_location, ship.ship_type.to_owned()).await?;
                    current_user_info = scout.client.get_user_info().await?;
                } else {
                    panic!("Unable to find a ship for the user to purchase and the user doesn't currently have any ships");
                }
            }

            println!("Scout {} -- Found {} ships for user {}", scout.username, current_user_info.user.ships.len(), scout.username);
            if !current_user_info.user.ships.is_empty() {
                let mut ship = current_user_info.user.ships.get(0).unwrap();
                let assigned_location = scout.location_symbol.clone().unwrap();
                let system_location = scout.client.get_location_info(assigned_location.clone()).await?;

                // If the ship is currently in motion then look up it's flight plan and wait for
                // the remaining type before continuing
                if ship.location == None {
                    println!("Scout {} -- is currently in motion...", scout.username);

                    // search for any stored flight plans that are valid for this scout.
                    let flight_plan = db::get_active_flight_plan(pg_pool.clone(), ship)
                        .await.expect("Unable to get active flight plans");

                    if let Some(flight_plan) = flight_plan {
                        println!("Scout {} -- current flight plan {:?}", scout.username, flight_plan);

                        // Adding 5 seconds here just to give the flight plan a little buffer
                        let remaining_seconds = (flight_plan.arrives_at - Utc::now()).num_seconds() + 5;

                        println!("Scout {} -- {} seconds remaining in flight plan... waiting", scout.username, remaining_seconds);
                        if remaining_seconds > 0 {
                            tokio::time::sleep(Duration::from_secs(
                                u64::try_from(remaining_seconds).expect("Invalid remaining seconds encountered")
                            )).await;
                        }

                        current_user_info = scout.client.get_user_info().await?;
                        ship = current_user_info.user.ships.get(0).unwrap();
                    }
                }

                // if the scout isn't at it's assigned location then send it there
                if ship.location.clone() != scout.location_symbol {
                    println!("Scout {} -- moving to location {}", scout.username, assigned_location);

                    // If the ship has any space available fill it up with fuel
                    if ship.space_available > 0 {
                        println!("Scout {} -- filling ship with {} fuel", scout.username, ship.space_available);
                        scout.client.create_purchase_order(ship.to_owned(), Good::Fuel, ship.space_available).await?;
                    }

                    let flight_plan = funcs::create_flight_plan(
                        &scout.client,
                        pg_pool.clone(),
                        ship,
                        scout.location_symbol.unwrap())
                        .await
                        .expect("Unable to create flight plan");

                    let flight_seconds = flight_plan.flight_plan.time_remaining_in_seconds + 5;
                    println!("Scout {} -- waiting for {} seconds", scout.username, flight_seconds);

                    tokio::time::sleep(Duration::from_secs(flight_seconds.try_into().unwrap())).await;

                    println!("Scout {} -- arrived at {}", scout.username, assigned_location);
                }

                // now start collecting marketplace data every 10 minutes
                loop {
                    println!("Scout {} -- is at {} harvesting marketplace data", scout.username, assigned_location);

                    let marketplace_data = scout.client.get_location_marketplace(assigned_location.clone()).await?;
                    println!("Scout {} -- at {} received marketplace data {:?}", scout.username, assigned_location, marketplace_data);

                    for datum in marketplace_data.location.marketplace {
                        db::persist_market_data(pg_pool.clone(), &system_location.location, &datum)
                            .await.expect("Unable to save market data");
                    }

                    println!("Scout {} -- is waiting for 10 minutes to get another round of data", scout.username);
                    tokio::time::sleep(Duration::from_secs(60 * 10)).await;
                }
            }

            Ok::<(), Box<dyn std::error::Error + Send>>(())
        }));
    }

    futures::future::join_all(handles).await;

    Ok(())
}
