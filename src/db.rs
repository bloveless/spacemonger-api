use spacetraders::{shared, responses};
use tokio_postgres::{Client, NoTls, Error};

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("migrations");
}

#[derive(Debug)]
pub struct User {
    pub id: String,
    pub username: String,
    pub token: String,
}

pub async fn get_client(host: String, username: String, password: String, database: String) -> Result<Client, Error> {
    let (client, connection) = tokio_postgres::connect(&format!("host={} user={} password={} dbname={}", host, username, password, database), NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    Ok(client)
}

pub async fn run_migrations(client: &mut Client) -> Result<(), Box<dyn std::error::Error>> {
    let migration_report = embedded::migrations::runner().run_async(client).await?;

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

pub async fn get_current_user(client: &mut Client) -> Result<User, Error> {
    let result = client.query_one("
        SELECT id, username, token FROM users WHERE active = TRUE LIMIT 1;
    ", &[]).await?;

    Ok(
        User {
            id: result.get("id"),
            username: result.get("username"),
            token: result.get("token"),
        }
    )
}

pub async fn truncate_system_info(client: &mut Client) -> Result<u64, Error> {
    Ok(client.execute("DELETE FROM system_info;", &[]).await?)
}

pub async fn persist_system_location(client: &mut Client, system: &shared::SystemsInfoData, location: &shared::SystemsInfoLocation) -> Result<u64, Error> {
    Ok(
        client.execute("
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

pub async fn persist_flight_plan(client: &mut Client, ship: &shared::Ship, flight_plan: &responses::FlightPlan) -> Result<u64, Error> {
    Ok(
        client.execute("
            INSERT INTO flight_plans (ship_id, flight_plan_id, origin, destination, ship_cargo_volume, ship_cargo_volume_max, distance, fuel_consumed, fuel_remaining, time_remaining_in_seconds, arrives_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11);
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
        ]).await?
    )
}

pub async fn persist_market_data(client: &mut Client, location: &shared::SystemsInfoLocation, marketplace_data: &shared::MarketplaceData) -> Result<u64, Error> {
    Ok(
        client.execute("
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
