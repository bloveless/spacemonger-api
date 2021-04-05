use spacetraders::{shared, responses};
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{Pool, Postgres, Row};

pub type PgPool = Pool<Postgres>;

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

pub async fn get_db_pool(host: String, port: i32, username: String, password: String, database: String) -> Result<PgPool, Box<dyn std::error::Error>> {
    let pg_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&format!("postgresql://{}:{}@{}:{}/{}", username, password, host, port, database))
        .await?;

    Ok(pg_pool)
}

pub async fn run_migrations(pg_pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::migrate!("./migrations")
        .run(&pg_pool)
        .await
        .expect("Failed to migrate database");

    Ok(())
}

pub async fn get_user(pg_pool: PgPool, username: String) -> Result<Option<User>, Box<dyn std::error::Error>> {
    Ok(
        sqlx::query("
            SELECT id::text, username, token, assignment, location FROM users
            WHERE username = $1
            LIMIT 1;
        ")
            .bind(&username)
            .map(|row: PgRow| {
                User {
                    id: row.get("id"),
                    username: row.get("username"),
                    token: row.get("token"),
                    assignment: row.get("assignment"),
                    location: row.get("location"),
                }
            })
            .fetch_optional(&pg_pool)
            .await?
    )
}

pub async fn persist_user(pg_pool: PgPool, username: String, token: String, assignment: String, location: Option<String>) -> Result<User, Box<dyn std::error::Error>> {
    Ok(
        sqlx::query("
            INSERT INTO users (username, token, assignment, location)
            VALUES ($1, $2, $3, $4)
            RETURNING id::text, username, token, assignment, location;
        ")
            .bind(&username)
            .bind(&token)
            .bind(&assignment)
            .bind(&location)
            .map(|row: PgRow| {
                User {
                    id: row.get("id"),
                    username: row.get("username"),
                    token: row.get("token"),
                    assignment: row.get("assignment"),
                    location: row.get("location"),
                }
            })
            .fetch_one(&pg_pool)
            .await?
    )
}

pub async fn truncate_system_info(pg_pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query("DELETE FROM system_info;").execute(&pg_pool).await?;

    Ok(())
}

pub async fn persist_system_location(pg_pool: PgPool, system: &shared::SystemsInfoData, location: &shared::SystemsInfoLocation) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query("
        INSERT INTO system_info(system, system_name, location, location_name, location_type, x, y)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT ON CONSTRAINT unique_system_info_system_location
        DO UPDATE SET
            system_name = $2,
            location_name = $4,
            location_type = $5,
            x = $6,
            y = $7;
    ")
        .bind(&system.symbol)
        .bind(&system.name)
        .bind(&location.symbol)
        .bind(&location.name)
        .bind(&location.systems_info_type.to_string())
        .bind(&location.x)
        .bind(&location.y)
        .execute(&pg_pool)
        .await?;

    Ok(())
}

pub async fn persist_flight_plan(pg_pool: PgPool, user_id: String, ship: &shared::Ship, flight_plan: &responses::FlightPlan) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query("
        INSERT INTO flight_plans (ship_id, flight_plan_id, origin, destination, ship_cargo_volume, ship_cargo_volume_max, distance, fuel_consumed, fuel_remaining, time_remaining_in_seconds, arrives_at, user_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12);
    ")
        .bind(&ship.id)
        .bind(&flight_plan.flight_plan.id)
        .bind(&flight_plan.flight_plan.departure)
        .bind(&flight_plan.flight_plan.destination)
        .bind(&ship.cargo.iter().fold(0, |sum, cargo| sum + cargo.total_volume))
        .bind(&ship.max_cargo)
        .bind(&flight_plan.flight_plan.distance)
        .bind(&flight_plan.flight_plan.fuel_consumed)
        .bind(&flight_plan.flight_plan.fuel_remaining)
        .bind(&flight_plan.flight_plan.time_remaining_in_seconds)
        .bind(&flight_plan.flight_plan.arrives_at)
        .bind(&user_id)
        .execute(&pg_pool)
        .await?;

    Ok(())
}

pub async fn persist_market_data(pg_pool: PgPool, location: &shared::SystemsInfoLocation, marketplace_data: &shared::MarketplaceData) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query("
        INSERT INTO market_data(planet_symbol, good_symbol, price_per_unit, volume_per_unit, available)
        VALUES ($1, $2, $3, $4, $5);
    ")
        .bind(&location.symbol)
        .bind(&marketplace_data.symbol.to_string())
        .bind(&marketplace_data.price_per_unit)
        .bind(&marketplace_data.volume_per_unit)
        .bind(&marketplace_data.quantity_available)
        .execute(&pg_pool)
        .await?;

    Ok(())
}
