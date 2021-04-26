use crate::db;
use spacetraders::client::{Client, HttpClient};
use spacetraders::{shared, responses, client};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct User {
    pub username: String,
    pub id: String,
    pub assignment: String,
    pub system_symbol: Option<String>,
    pub location_symbol: Option<String>,
    pub client: Client,
}

pub(crate) async fn get_user(http_client: HttpClient, pg_pool: PgPool, username: String, assignment: String, system_symbol: Option<String>, location_symbol: Option<String>) -> Result<User, Box<dyn std::error::Error>> {
    let db_user = db::get_user(pg_pool.clone(), username.to_owned()).await?;

    if let Some(user) = db_user {
        println!("Found existing user {}", username);
        Ok(
            User {
                username,
                id: user.id,
                assignment,
                system_symbol,
                location_symbol,
                client: Client::new(http_client, user.username, user.token),
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
                id: user.id,
                assignment,
                system_symbol,
                location_symbol,
                client: Client::new(http_client.clone(), username.to_owned(), claimed_user.token.to_owned()),
            }
        )
    }
}


pub async fn create_flight_plan(user: &User, pg_pool: PgPool, ship: &shared::Ship, destination: String) -> Result<responses::FlightPlan, Box<dyn std::error::Error>> {
    let flight_plan = user.client.create_flight_plan(ship.id.to_owned(), destination.to_owned()).await?;

    db::persist_flight_plan(pg_pool, user.id.clone(), ship, &flight_plan).await?;

    Ok(flight_plan)
}

pub async fn get_systems(user: &User, pg_pool: PgPool) -> Result<responses::SystemsInfo, Box<dyn std::error::Error>> {
    let systems_info = user.client.get_systems_info().await?;
    println!("Systems info: {:?}", systems_info);

    for system in &systems_info.systems {
        for location in &system.locations {
            db::persist_system_location(pg_pool.clone(), system, location).await?;
        }
    }

    Ok(systems_info)
}