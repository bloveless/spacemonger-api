use spacetraders::{shared, responses};
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{Row, PgPool};
use chrono::{Utc, Datelike};

#[derive(Debug)]
pub struct Ship {
    pub id: String,
    pub username: String,
    pub token: String,
}

#[derive(Debug)]
pub struct DbUser {
    pub id: String,
    pub username: String,
    pub token: String,
    pub assignment: String,
    pub system_symbol: Option<String>,
    pub location_symbol: Option<String>,
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

pub async fn reset_db(pg_pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let now = Utc::now();
    let now = now.format("%Y%m%d").to_string();

    sqlx::query(&format!("ALTER TABLE daemon_flight_plans RENAME TO z{}_daemon_flight_plans", now))
        .execute(&pg_pool).await?;
    sqlx::query(&format!("ALTER TABLE daemon_market_data RENAME TO z{}_daemon_market_data", now))
        .execute(&pg_pool).await?;
    sqlx::query(&format!("ALTER TABLE daemon_system_info RENAME TO z{}_daemon_system_info", now))
        .execute(&pg_pool).await?;
    sqlx::query(&format!("ALTER TABLE daemon_users RENAME TO z{}_daemon_users", now))
        .execute(&pg_pool).await?;

    // Drop the sqlx migrations table so all the tables will be created again
    sqlx::query("DROP TABLE _sqlx_migrations;").execute(&pg_pool).await?;

    Ok(())
}

pub async fn get_user(pg_pool: PgPool, username: String) -> Result<Option<DbUser>, Box<dyn std::error::Error>> {
    Ok(
        sqlx::query("
            SELECT id::text, username, token, assignment, system_symbol, location_symbol FROM daemon_users
            WHERE username = $1
            LIMIT 1;
        ")
            .bind(&username)
            .map(|row: PgRow| {
                DbUser {
                    id: row.get("id"),
                    username: row.get("username"),
                    token: row.get("token"),
                    assignment: row.get("assignment"),
                    system_symbol: row.get("system_symbol"),
                    location_symbol: row.get("location_symbol"),
                }
            })
            .fetch_optional(&pg_pool)
            .await?
    )
}

pub async fn persist_user(pg_pool: PgPool, username: String, token: String, assignment: String, system_symbol: Option<String>, location_symbol: Option<String>) -> Result<DbUser, Box<dyn std::error::Error>> {
    Ok(
        sqlx::query("
            INSERT INTO daemon_users (username, token, assignment, system_symbol, location_symbol)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id::text, username, token, assignment, system_symbol, location_symbol;
        ")
            .bind(&username)
            .bind(&token)
            .bind(&assignment)
            .bind(&system_symbol)
            .bind(&location_symbol)
            .map(|row: PgRow| {
                DbUser {
                    id: row.get("id"),
                    username: row.get("username"),
                    token: row.get("token"),
                    assignment: row.get("assignment"),
                    system_symbol: row.get("system_symbol"),
                    location_symbol: row.get("location_symbol"),
                }
            })
            .fetch_one(&pg_pool)
            .await?
    )
}

pub async fn persist_system_location(pg_pool: PgPool, system: &shared::SystemsInfoData, location: &shared::SystemsInfoLocation) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query("
        INSERT INTO daemon_system_info(system_symbol, system_name, location_symbol, location_name, location_type, x, y)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (system_symbol, location_symbol)
        DO UPDATE SET
            system_name = $2,
            location_name = $4,
            location_type = $5,
            x = $6,
            y = $7,
            created_at = timezone('utc', NOW());
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
        INSERT INTO daemon_flight_plans (
             id
            ,user_id
            ,ship_id
            ,origin
            ,destination
            ,ship_cargo_volume
            ,ship_cargo_volume_max
            ,distance
            ,fuel_consumed
            ,fuel_remaining
            ,time_remaining_in_seconds
            ,arrives_at
        ) VALUES ($1, uuid($2), $3, $4, $5, $6, $7, $8, $9, $10, $11, $12);
    ")
        .bind(&flight_plan.flight_plan.id)
        .bind(&user_id)
        .bind(&ship.id)
        .bind(&flight_plan.flight_plan.departure)
        .bind(&flight_plan.flight_plan.destination)
        .bind(&ship.cargo.iter().fold(0, |sum, cargo| sum + cargo.total_volume))
        .bind(&ship.max_cargo)
        .bind(&flight_plan.flight_plan.distance)
        .bind(&flight_plan.flight_plan.fuel_consumed)
        .bind(&flight_plan.flight_plan.fuel_remaining)
        .bind(&flight_plan.flight_plan.time_remaining_in_seconds)
        .bind(&flight_plan.flight_plan.arrives_at)
        .execute(&pg_pool)
        .await?;

    Ok(())
}

pub async fn get_active_flight_plan(pg_pool: PgPool, ship: &shared::Ship) -> Result<Option<shared::FlightPlanData>, Box<dyn std::error::Error>> {
    Ok(
        sqlx::query("
            SELECT
                 id
                ,ship_id
                ,origin
                ,destination
                ,fuel_consumed
                ,fuel_remaining
                ,time_remaining_in_seconds
                ,created_at
                ,distance
                ,arrives_at
                ,user_id
            FROM daemon_flight_plans
            WHERE ship_id = $1
                AND arrives_at > $2
        ")
            .bind(&ship.id)
            .bind(&Utc::now())
            .map(|row: PgRow| {
                shared::FlightPlanData {
                    id: row.get("id"),
                    ship_id: row.get("ship_id"),
                    fuel_consumed: row.get("fuel_consumed"),
                    fuel_remaining: row.get("fuel_remaining"),
                    time_remaining_in_seconds: row.get("time_remaining_in_seconds"),
                    created_at: row.get("created_at"),
                    arrives_at: row.get("arrives_at"),
                    terminated_at: None,
                    destination: row.get("destination"),
                    departure: row.get("origin"),
                    distance: row.get("distance"),
                }
            })
            .fetch_optional(&pg_pool)
            .await?
    )
}

pub async fn persist_market_data(pg_pool: PgPool, location: &shared::SystemsInfoLocation, marketplace_data: &shared::MarketplaceData) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query("
        INSERT INTO daemon_market_data(location_symbol, good_symbol, price_per_unit, volume_per_unit, quantity_available, purchase_price_per_unit, sell_price_per_unit)
        VALUES ($1, $2, $3, $4, $5, $6, $7);
    ")
        .bind(&location.symbol)
        .bind(&marketplace_data.symbol.to_string())
        .bind(&marketplace_data.price_per_unit)
        .bind(&marketplace_data.volume_per_unit)
        .bind(&marketplace_data.quantity_available)
        .bind(&marketplace_data.purchase_price_per_unit)
        .bind(&marketplace_data.sell_price_per_unit)
        .execute(&pg_pool)
        .await?;

    Ok(())
}
