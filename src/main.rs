mod funcs;
mod db;

use spacetraders::{client::Client, client};
use std::env;
use dotenv::dotenv;
use spacetraders::client::ClientRateLimiter;
use tokio::time::Duration;
use std::convert::{TryInto, TryFrom};
use spacetraders::shared::{LoanType, Good};
use chrono::Utc;

const BASE_ACCOUNT_NAME: &str = "bloveless-dev5";

#[derive(Debug)]
struct User {
    username: String,
    assignment: String,
    location: Option<String>,
    client: Client,
}

async fn get_user(client_rate_limiter: ClientRateLimiter, pg_pool: db::PgPool, username: String, assignment: String, location: Option<String>) -> Result<User, Box<dyn std::error::Error>> {
    let db_user = db::get_user(pg_pool.clone(), username.to_owned()).await?;

    if let Some(user) = db_user {
        println!("Found existing user {}", username);
        Ok(
            User {
                username,
                assignment,
                location,
                client: Client::new(client_rate_limiter, user.id, user.username, user.token),
            }
        )
    } else {
        println!("Creating new user {}", username);
        let claimed_user = client::claim_username(username.to_owned()).await?;

        println!("Claimed new user {:?}", claimed_user);

        let user = db::persist_user(
            pg_pool.clone(),
            username.to_owned(),
            claimed_user.token.to_owned(),
            assignment.to_owned(),
            location.to_owned()
        ).await?;

        println!("New user persisted");

        Ok(
            User {
                username: username.to_owned(),
                assignment,
                location,
                client: Client::new(client_rate_limiter, user.id, username.to_owned(), claimed_user.token.to_owned()),
            }
        )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    // let args: Vec<String> = env::args().collect();
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

    let game_rate_limiter = client::get_rate_limiter();

    let main_user = get_user(game_rate_limiter.clone(), pg_pool.clone(), format!("{}-main", BASE_ACCOUNT_NAME), "main".to_string(), None).await?;

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

    let mut scouts: Vec<User> = Vec::new();

    for system in &system_info.systems {
        // TODO: We only support one system right now
        if system.symbol == "XV" {
            for location in &system.locations {
                let scout_user = get_user(game_rate_limiter.clone(), pg_pool.clone(), format!("{}-scout-{}", BASE_ACCOUNT_NAME, location.symbol), "scout".to_string(), Some(location.symbol.to_owned())).await?;

                scouts.push(scout_user);

                // TODO: Remove this when we are ready to test more than one scout
                break;
            }
        }
    }

    println!("Main user info: {:?}",  main_user.client.get_user_info().await?);

    let mut handles = vec![];

    for scout in scouts {
        let pg_pool = pg_pool.clone();
        handles.push(tokio::spawn(async move {
            let mut current_user_info = scout.client.get_user_info().await?;

            // 1. if the user doesn't have enough credits take out a startup loan
            println!("Scout {} -- user info {:?}", scout.username, current_user_info);
            if current_user_info.user.credits == 0 {
                // assume that if the user has 0 credits that the user needs to take out a loan
                current_user_info = scout.client.request_new_loan(LoanType::Startup).await?;
            }

            // 2. if the user doesn't have any ships then buy the fastest one that the user can afford
            if current_user_info.user.ships.is_empty() {
                let available_ships = scout.client.get_ships_for_sale().await?;
                let mut fastest_ship = None;
                let mut fastest_ship_speed = 0;
                let mut fastest_ship_location = "".to_string();
                let mut fastest_ship_price = 0;

                for available_ship in &available_ships.ships {
                    for purchase_location in &available_ship.purchase_locations {
                        if available_ship.speed > fastest_ship_speed && current_user_info.user.credits > purchase_location.price {
                            fastest_ship_speed = available_ship.speed;
                            fastest_ship = Some(available_ship);
                            fastest_ship_location = purchase_location.location.to_owned();
                            fastest_ship_price = purchase_location.price;
                        }
                    }
                }

                if let Some(ship) = fastest_ship {
                    println!("Scout {} -- Buying {} for {} at location {}", scout.username, ship.ship_type.to_owned(), fastest_ship_price, fastest_ship_location);
                    current_user_info = scout.client.purchase_ship(fastest_ship_location, ship.ship_type.to_owned()).await?;
                } else {
                    panic!("Unable to find a ship for the user to purchase and the user doesn't currently have any ships");
                }
            }

            println!("Scout {} -- Found {} ships for user {}", scout.username, current_user_info.user.ships.len(), scout.username);
            if !current_user_info.user.ships.is_empty() {
                let mut ship = current_user_info.user.ships.get(0).unwrap();
                let assigned_location = scout.location.clone().unwrap();
                let system_location = scout.client.get_location_info(assigned_location.clone()).await?;

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
                // TODO: the ship could be currently moving if I've restarted
                //       I should look up the flight plan and see if the ship is in flight
                if ship.location.clone() != scout.location {
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
                        scout.location.unwrap())
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
