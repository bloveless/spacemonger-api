mod funcs;
mod db;

use spacetraders::{client::Client, client};
use std::env;
use dotenv::dotenv;
use tokio_postgres::Client as PgClient;
use std::time::Duration;
use std::sync::Arc;

const BASE_ACCOUNT_NAME: &str = "bloveless";

type ClientRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

async fn get_client_for_user(client_rate_limiter: ClientRateLimiter, pg_client: &mut PgClient, username: String, assignment: String, location: Option<String>) -> Result<Client, Box<dyn std::error::Error>> {
    let db_user = db::get_user(pg_client, username.to_owned()).await?;

    if let Some(user) = db_user {
        println!("Found existing user {}", username);
        Ok(Client::new(client_rate_limiter, user.id, user.username, user.token))
    } else {
        println!("Creating new user {}", username);
        let claimed_user = client::claim_username(username.to_owned()).await?;

        println!("Claimed new user {:?}", claimed_user);

        let user = db::persist_user(
            pg_client,
            username.to_owned(),
            claimed_user.token.to_owned(),
            assignment.to_owned(),
            location.to_owned()
        ).await?;

        println!("New user persisted");

        Ok(Client::new(client_rate_limiter, user.id.to_owned(), username.to_owned(), claimed_user.token.to_owned()))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    // let args: Vec<String> = env::args().collect();
    let postgres_host = env::var("POSTGRES_HOST").unwrap();
    let postgres_username = env::var("POSTGRES_USERNAME").unwrap();
    let postgres_password = env::var("POSTGRES_PASSWORD").unwrap();
    let postgres_database = env::var("POSTGRES_DATABASE").unwrap();

    let mut pg_client = db::get_client(postgres_host, 5433, postgres_username, postgres_password, postgres_database).await?;

    db::run_migrations(&mut pg_client).await?;

    // Algorithm. Create the main user account (or get from db). Get the number of locations
    // in the system. Create (or get from db) X scout accounts (where X is number of locations in
    // the system). Send each scout account to the location they are assigned.

    let game_rate_limiter = Arc::new(client::get_rate_limiter());

    let main_user = get_client_for_user(game_rate_limiter.clone(), &mut pg_client, format!("{}-main", BASE_ACCOUNT_NAME), "main".to_string(), None).await?;

    let system_info = main_user.get_systems_info().await?;

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

    let mut scouts: Vec<Client> = Vec::new();

    for system in &system_info.systems {
        for location in &system.locations {
            println!("Create user {}-scout-{}", BASE_ACCOUNT_NAME, location.symbol);
            let scout_user = get_client_for_user(game_rate_limiter.clone(), &mut pg_client, format!("{}-scout-{}", BASE_ACCOUNT_NAME, location.symbol), "scout".to_string(), Some(location.symbol.to_owned())).await?;

            scouts.push(scout_user);
        }
    }

    println!("Scout Users: {:?}", scouts);

    println!("Main user info: {:?}",  main_user.get_user_info().await?);

    for scout in scouts {
        println!("Scout user info: {:?}",  scout.get_user_info().await?);
    }

    Ok(())
}
