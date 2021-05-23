mod funcs;
mod db;
mod user;
mod ship_machine;

use spacetraders::client;
use std::env;
use dotenv::dotenv;
use tokio::time::Duration;
use spacetraders::shared::LoanType;
use crate::ship_machine::{ShipAssignment, PollResult};
use spacetraders::errors::SpaceTradersClientError;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    env_logger::init();

    let username_base = env::var("USERNAME_BASE").unwrap();
    let postgres_host = env::var("POSTGRES_HOST").unwrap();
    let postgres_port = env::var("POSTGRES_PORT").unwrap().parse::<i32>().unwrap();
    let postgres_username = env::var("POSTGRES_USERNAME").unwrap();
    let postgres_password = env::var("POSTGRES_PASSWORD").unwrap();
    let postgres_database = env::var("POSTGRES_DATABASE").unwrap();
    let enable_scouts = env::var("ENABLE_SCOUTS").unwrap().parse::<bool>().unwrap();
    let enable_reset = env::var("ENABLE_RESET").unwrap().parse::<bool>().unwrap();

    let pg_pool = db::get_db_pool(postgres_host, postgres_port, postgres_username, postgres_password, postgres_database).await?;

    db::run_migrations(pg_pool.clone()).await?;

    // Algorithm. Create the main user account (or get from db). Get the number of locations
    // in the system. Create (or get from db) X scout accounts (where X is number of locations in
    // the system). Send each scout account to the location they are assigned.

    let http_client = client::get_http_client();

    if funcs::is_api_in_maintenance_mode(http_client.clone()).await {
        loop {
            log::warn!("Detected SpaceTraders API in maintenance mode (status code 503). Sleeping for 60 seconds and trying again");
            tokio::time::sleep(Duration::from_secs(60)).await;

            if !funcs::is_api_in_maintenance_mode(http_client.clone()).await {
                break;
            }
        }
    }

    // When an API reset occurs all the scouts will being to fail making requests.
    // As soon as all the scouts fail this pod will restart. Upon restart we will check
    // if the API is in maintenance mode (status code 503) if it is then we will wait for
    // maintenance mode to end. After that ends if the main user is unable to make a requests
    // we can assume that the API has been reset and we need to reset ourselves.
    let user = user::User::new(
        http_client.clone(),
        pg_pool.clone(),
        format!("{}-main", username_base),
        ShipAssignment::Trader,
    ).await;

    if user.is_err() {
        if enable_reset {
            db::reset_db(pg_pool.clone()).await?;
        }
        // Now that the tables have been moved we will panic so that the pod will restart and the tables will be recreated
        panic!("Unable to connect using the main user. Assuming an API reset. Backing up data and clearing the database");
    }

    let mut user = user.unwrap();

    let system_info = user.get_systems().await?;

    for system in &system_info.systems {
        for location in &system.locations {
            db::persist_system_location(pg_pool.clone(), system, location).await?;
        }
    }

    log::info!("## Begin System Messages ----------------------------------------------------------");
    for system in &system_info.systems {
        for location in &system.locations {
            if let Some(messages) = &location.messages {
                for message in messages {
                    log::info!("Location: {} Message: {}", location.symbol, message)
                }
            }
        }
    }
    log::info!("## End System Messages ------------------------------------------------------------");

    let mut users = Vec::new();
    let mut user_handles = Vec::new();

    if enable_scouts {
        for system in &system_info.systems {
            for location in &system.locations {
                let mut scout_user = user::User::new(
                    http_client.clone(),
                    pg_pool.clone(),
                    format!("{}-scout-{}", username_base, location.symbol),
                    ShipAssignment::Scout {
                        system_symbol: system.symbol.clone(),
                        location_symbol: location.symbol.clone(),
                    },
                ).await?;

                // 1. if the user doesn't have enough credits take out a startup loan
                log::info!("Scout {} -- credits {}", scout_user.username, scout_user.credits);
                if scout_user.credits == 0 {
                    log::info!("Scout {} -- Requesting new {:?} loan", scout_user.username, LoanType::Startup);
                    // assume that if the user has 0 credits that the user needs to take out a loan
                    scout_user.request_new_loan(LoanType::Startup).await?;
                }

                // 2. if the user doesn't have any ships then buy the fastest one that the user can afford that is in the system assigned to the scout
                if scout_user.ship_machines.is_empty() {
                    scout_user.purchase_fastest_ship(Some(&system.symbol)).await?;
                }

                users.push(scout_user);
            }
        }
    }

    // One task per user, each of those will create new tasks for each of it's ships
    // The main task will handle upgrades by checking the users credits and ships periodically
    // The main task will be able to create new ships and push them into the ship_handles array to
    // be awaited upon later
    // That's all that we need for creating new ships, but upgrading ships we need to be able to
    // notify a ship task that it needs to be upgraded

    // Setup our main user
    // 1. if the user doesn't have enough credits take out a startup loan
    if user.credits == 0 {
        log::info!("User {} -- Requesting new {:?} loan", user.username, LoanType::Startup);
        // assume that if the user has 0 credits that the user needs to take out a loan
        user.request_new_loan(LoanType::Startup).await?;
    }

    // 2. if the user doesn't have any ships then buy the largest one that the user can afford
    if user.ship_machines.is_empty() {
        user.purchase_largest_ship(None).await?;
    }

    users.push(user);

    for user in users {
        let mut user = user.clone();
        let pg_pool = pg_pool.clone();
        user_handles.push(tokio::spawn(async move {
            let mut prev_user_credits = 0;
            loop {
                for machine in &mut user.ship_machines {
                    match machine.poll().await {
                        Ok(poll_result) => {
                            // TODO: Maybe there will be some signals that come back from the tick
                            //       function that we should close and respawn the task... or handle errors
                            //       or something like that
                            if let Some(poll_result) = poll_result {
                                match poll_result {
                                    PollResult::UpdateCredits(credits) => user.credits = credits,
                                }
                            }
                        }
                        Err(e) => {
                            if let Some(e) = e.downcast_ref::<SpaceTradersClientError>() {
                                match e {
                                    // TODO: There is one thing that concerns me about this... if the service unavailable
                                    //       message lasts for less than three minutes then the pod could end up in
                                    //       a state where some of the tasks have panic'd and others have not which
                                    //       would keep the pod running... consider how to fix this... later
                                    SpaceTradersClientError::ServiceUnavailable => {
                                        panic!("Caught a service unavailable error. Restarting the pod");
                                    }
                                    SpaceTradersClientError::Unauthorized => {
                                        panic!("Api returned Unauthorized response. Restarting the pod");
                                    }
                                    // NOTE: All other errors we are just going to skip because the state machine
                                    //       will just try it again... which is fine
                                    // SpaceTradersClientError::Http(_) => {}
                                    // SpaceTradersClientError::ApiError(_) => {}
                                    // SpaceTradersClientError::TooManyRetries => {}
                                    // SpaceTradersClientError::JsonParse(_) => {}
                                    other_error => {
                                        log::error!("Caught a space traders client. Error: {}", other_error);
                                    }
                                }
                            }
                        }
                    }
                }

                if prev_user_credits != user.credits {
                    log::info!("{} -- Credits {}", user.username, user.credits);
                    prev_user_credits = user.credits;

                    let user_ships = user.get_my_ships().await.unwrap();
                    db::persist_user_stats(pg_pool.clone(), &user.id, user.credits, &user_ships.ships)
                        .await.unwrap();

                    // We want to keep a base amount of 500k but as we get more ships it is more
                    // costly to fill them with goods so we add 75k per ship to make sure we don't
                    // go broke
                    if user.credits > (500_000 + (user.ship_machines.len() as i32 * 75_000)) && user.ship_machines.len() < 100 {
                        match user.purchase_largest_ship(None).await {
                            Ok(_) => {}
                            Err(e) => log::error!("{} -- Error occurred while purchasing a ship. Error: {}", user.username, e)
                        };
                    }

                    // After we are millionaires we should probably pay off our loans
                    if user.credits > 1_000_000 && user.outstanding_loans > 0 {
                        let loan = user.loans.first().unwrap();
                        match user.pay_off_loan(&loan.id).await {
                            Ok(pay_loan_response) => {
                                log::info!("{} -- User paid off loan id {}", user.username, loan.id);
                                user.loans = pay_loan_response.user.loans.clone();
                                user.outstanding_loans = pay_loan_response.user.loans.into_iter().filter(|l| { !l.status.contains("PAID") }).count();
                                user.credits = pay_loan_response.user.credits;
                            }
                            Err(e) => log::error!("{} -- Unable to pay off loan id {}. Error: {}", user.username, loan.id, e)
                        }
                    }
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }));
    }

    futures::future::join_all(user_handles).await;

    Ok(())
}
