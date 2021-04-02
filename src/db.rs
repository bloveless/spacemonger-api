use spacetraders::{shared, responses};
use tokio_postgres::{Client as PgClient, NoTls, Error};

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("migrations");
}

#[derive(Debug)]
pub struct Ship {
    pub id: String,
    pub username: String,
    pub token: String,
}

#[derive(Debug)]
pub struct User {
    pub id: String,
    pub username: String,
    pub token: String,
    pub assignment: String,
    pub location: Option<String>,
}

pub async fn get_client(host: String, port: i32, username: String, password: String, database: String) -> Result<PgClient, Error> {
    let connection_url = format!("postgresql://{}:{}@{}:{}/{}", username, password, host, port, database);
    let (pg_client, connection) = tokio_postgres::connect(&connection_url, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    Ok(pg_client)
}

pub async fn run_migrations(pg_client: &mut PgClient) -> Result<(), Box<dyn std::error::Error>> {
    let migration_report = embedded::migrations::runner().run_async(pg_client).await?;

    for migration in migration_report.applied_migrations() {
        println!(
            "Migration Applied -  Name: {}, Version: {}",
            migration.name(),
            migration.version()
        );
    }

    println!("DB migrations finished");

    Ok(())
}

pub async fn get_user(pg_client: &mut PgClient, username: String) -> Result<Option<User>, Error> {
    let result = pg_client.query("
        SELECT id::text, username, token, assignment, location FROM users
        WHERE username = $1
        LIMIT 1;
    ", &[&username]).await?;

    Ok(
        if result.is_empty() {
            None
        } else {
            Some(
                User {
                    id: result[0].get("id"),
                    username: result[0].get("username"),
                    token: result[0].get("token"),
                    assignment: result[0].get("assignment"),
                    location: result[0].get("location"),
                }
            )
        }
    )
}

pub async fn persist_user(pg_client: &mut PgClient, username: String, token: String, assignment: String, location: Option<String>) -> Result<User, Error> {
    let result = pg_client.query_one("
        INSERT INTO users (username, token, assignment, location)
        VALUES ($1, $2, $3, $4)
        RETURNING id::text;
    ", &[&username, &token, &assignment, &location]).await?;

    Ok(
        User {
            id: result.get("id"),
            username,
            token,
            assignment,
            location
        }
    )
}

pub async fn truncate_system_info(pg_client: &mut PgClient) -> Result<u64, Error> {
    Ok(pg_client.execute("DELETE FROM system_info;", &[]).await?)
}

pub async fn persist_system_location(pg_client: &mut PgClient, system: &shared::SystemsInfoData, location: &shared::SystemsInfoLocation) -> Result<u64, Error> {
    Ok(
        pg_client.execute("
                INSERT INTO system_info(system, system_name, location, location_name, location_type, x, y)
                VALUES ($1, $2, $3, $4, $5, $6, $7);
            ", &[
            &system.symbol,
            &system.name,
            &location.symbol,
            &location.name,
            &location.systems_info_type.to_string(),
            &location.x,
            &location.y,
        ],
        ).await?
    )
}

pub async fn persist_flight_plan(pg_client: &mut PgClient, user_id: String, ship: &shared::Ship, flight_plan: &responses::FlightPlan) -> Result<u64, Error> {
    Ok(
        pg_client.execute("
            INSERT INTO flight_plans (ship_id, flight_plan_id, origin, destination, ship_cargo_volume, ship_cargo_volume_max, distance, fuel_consumed, fuel_remaining, time_remaining_in_seconds, arrives_at, user_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12);
        ", &[
            &ship.id,
            &flight_plan.flight_plan.id,
            &flight_plan.flight_plan.departure,
            &flight_plan.flight_plan.destination,
            &ship.cargo.iter().fold(0, |sum, cargo| sum + cargo.total_volume),
            &ship.max_cargo,
            &flight_plan.flight_plan.distance,
            &flight_plan.flight_plan.fuel_consumed,
            &flight_plan.flight_plan.fuel_remaining,
            &flight_plan.flight_plan.time_remaining_in_seconds,
            &flight_plan.flight_plan.arrives_at,
            &user_id,
        ]).await?
    )
}

pub async fn persist_market_data(pg_client: &mut PgClient, location: &shared::SystemsInfoLocation, marketplace_data: &shared::MarketplaceData) -> Result<u64, Error> {
    Ok(
        pg_client.execute("
            INSERT INTO market_data(planet_symbol, good_symbol, price_per_unit, volume_per_unit, available)
            VALUES ($1, $2, $3, $4, $5);
        ", &[
            &location.symbol,
            &marketplace_data.symbol.to_string(),
            &marketplace_data.price_per_unit,
            &marketplace_data.volume_per_unit,
            &marketplace_data.quantity_available,
        ]).await?
    )
}
