mod funcs;
mod db;
mod user;

use spacetraders::client;
use std::env;
use dotenv::dotenv;
use tokio::time::Duration;
use std::convert::{TryInto, TryFrom};
use spacetraders::shared::{LoanType, Good};
use chrono::Utc;
use crate::user::User;
use spacetraders::errors::GameStatusError;

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

    if funcs::is_api_in_maintenance_mode(http_client.clone()).await {
        loop {
            println!("Detected SpaceTraders API in maintenance mode (status code 503). Sleeping for 60 seconds and trying again");
            tokio::time::sleep(Duration::from_secs(60)).await;

            if !funcs::is_api_in_maintenance_mode(http_client.clone()).await {
                break;
            }
        }
    }

    let mut main_user = User::new(http_client.clone(), pg_pool.clone(), format!("{}-main", username_base), "main".to_string(), None, None).await?;

    // When an API reset occurs all the scouts will being to fail making requests.
    // As soon as all the scouts fail this pod will restart. Upon restart we will check
    // if the API is in maintenance mode (status code 503) if it is then we will wait for
    // maintenance mode to end. After that ends if the main user is unable to make a requests
    // we can assume that the API has been reset and we need to reset ourselves.
    if main_user.update_user_info().await.is_err() {
        db::reset_db(pg_pool.clone()).await?;
        // Now that the tables have been we will panic so that the pod will restart and the tables will be recreated
        panic!("Unable to connect using the main user. Assuming an API reset. Backing up data and clearing the database");
    };

    let system_info = main_user.get_systems().await?;

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
        for location in &system.locations {
            let scout_user = User::new(
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

    main_user.update_user_info().await;
    println!("Main user info: {:?}", main_user.info);

    let mut handles = Vec::new();

    for scout in scouts {
        let pg_pool = pg_pool.clone();
        handles.push(tokio::spawn(async move {
            let mut scout = scout.clone();
            let assigned_location = scout.location_symbol.clone().unwrap();
            // 1. if the user doesn't have enough credits take out a startup loan
            println!("Scout {} -- user info {:?}", scout.username, scout.info);
            if scout.info.user.credits == 0 {
                println!("Scout {} -- Requesting new {:?} loan", scout.username, LoanType::Startup);
                // assume that if the user has 0 credits that the user needs to take out a loan
                scout.request_new_loan(LoanType::Startup).await?;
            }

            // 2. if the user doesn't have any ships then buy the fastest one that the user can afford that is in the system assigned to the scout
            if scout.info.user.ships.is_empty() {
                scout.purchase_fastest_ship().await?;
            }

            println!("Scout {} -- Found {} ships for user {}", scout.username, scout.info.user.ships.len(), scout.username);
            if !scout.info.user.ships.is_empty() {
                scout.maybe_wait_for_ship_to_arrive(0).await?;

                // if the scout isn't at it's assigned location then send it there
                scout.send_ship_to_location(pg_pool.clone(), 0, assigned_location.clone())
                    .await.expect("Unable to send ship to location");

                // now start collecting marketplace data every 10 minutes
                loop {
                    println!("Scout {} -- is at {} harvesting marketplace data", scout.username, assigned_location.clone());

                    scout.update_marketplace_data().await?;

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
