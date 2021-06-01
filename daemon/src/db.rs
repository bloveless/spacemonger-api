use serde::Serialize;
use spacetraders::{shared, responses};
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{Row, PgPool};
use chrono::{Utc, DateTime};
use std::cmp::Ordering::Equal;
use spacetraders::shared::Good;
use std::collections::HashMap;
use spacetraders::errors::SpaceTradersClientError;
use crate::ship_machines::ShipAssignment;

#[derive(Debug, Clone)]
pub struct Ship {
    pub id: String,
    pub username: String,
    pub token: String,
}

#[derive(Debug, Clone)]
pub struct DbUser {
    pub id: String,
    pub username: String,
    pub token: String,
    pub assignment: String,
    pub system_symbol: Option<String>,
    pub location_symbol: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DbSystemLocation {
    pub system_symbol: String,
    pub system_name: String,
    pub location_symbol: String,
    pub location_name: String,
    pub location_type: String,
    pub x: i32,
    pub y: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DbRoute {
    pub purchase_location_symbol: String,
    pub purchase_location_type: String,
    pub sell_location_symbol: String,
    pub good: Good,
    pub distance: f64,
    pub purchase_quantity: i32,
    pub sell_quantity: i32,
    pub purchase_price_per_unit: i32,
    pub sell_price_per_unit: i32,
    pub volume_per_unit: i32,
    pub fuel_required: f64,
    pub flight_time: f64,
    pub cost_volume_distance: f64,
    pub profit_speed_volume_distance: f64,
}

#[derive(Debug, Clone)]
pub struct DbDistanceBetweenLocations {
    pub origin_location_type: String,
    pub distance: f64,
}

pub async fn get_db_pool(host: String, port: i32, username: String, password: String, database: String) -> anyhow::Result<PgPool> {
    let pg_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&format!("postgresql://{}:{}@{}:{}/{}", username, password, host, port, database))
        .await?;

    Ok(pg_pool)
}

pub async fn run_migrations(pg_pool: PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations")
        .run(&pg_pool)
        .await
        .expect("Failed to migrate database");

    Ok(())
}

pub async fn reset_db(pg_pool: PgPool) -> anyhow::Result<()> {
    let now = Utc::now();
    let now = now.format("z%Y%m%d").to_string();

    #[derive(Debug)]
    struct DbTables {
        schema: String,
        name: String,
    }

    let results = sqlx::query("
        SELECT *
        FROM pg_catalog.pg_tables
        WHERE schemaname = 'public' AND tableowner = 'spacemonger';
    ")
        .map(|row: PgRow| {
            DbTables {
                schema: row.get("schemaname"),
                name: row.get("tablename"),
            }
        })
        .fetch_all(&pg_pool)
        .await?;

    sqlx::query(&format!("CREATE SCHEMA {}", now)).execute(&pg_pool).await?;

    for table in results {
        sqlx::query(&format!("ALTER TABLE {}.{} SET SCHEMA {}", table.schema, table.name, now))
            .execute(&pg_pool)
            .await?;
    }

    Ok(())
}

pub async fn get_user(pg_pool: PgPool, username: String) -> anyhow::Result<Option<DbUser>> {
    Ok(
        sqlx::query("
            SELECT id::text, username, token, assignment, system_symbol, location_symbol FROM daemon_user
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

pub async fn persist_user(pg_pool: PgPool, username: String, token: String, assignment: ShipAssignment) -> anyhow::Result<DbUser> {
    let assignment_type;
    let assignment_system_symbol;
    let assignment_location_symbol;
    match assignment {
        ShipAssignment::Scout { system_symbol, location_symbol } => {
            assignment_type = "scout";
            assignment_system_symbol = Some(system_symbol);
            assignment_location_symbol = Some(location_symbol);
        }
        ShipAssignment::Trader => {
            assignment_type = "trader";
            assignment_system_symbol = None;
            assignment_location_symbol = None;
        }
    }

    Ok(
        sqlx::query("
            INSERT INTO daemon_user (username, token, assignment, system_symbol, location_symbol)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id::text, username, token, assignment, system_symbol, location_symbol;
        ")
            .bind(&username)
            .bind(&token)
            .bind(&assignment_type)
            .bind(&assignment_system_symbol)
            .bind(&assignment_location_symbol)
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

pub async fn persist_system_location(pg_pool: PgPool, system: &shared::SystemsInfoData, location: &shared::SystemsInfoLocation) -> anyhow::Result<()> {
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

pub async fn get_system_locations_from_location(pg_pool: PgPool, location_symbol: &str) -> anyhow::Result<Vec<String>> {
    Ok(
        sqlx::query("
            SELECT
                dsi.location_symbol
            FROM daemon_system_info dsi
            INNER JOIN daemon_system_info dsi2
                ON dsi.system_symbol = dsi2.system_symbol
            WHERE dsi2.location_symbol = $1;
        ")
            .bind(location_symbol)
            .map(|row: PgRow| {
                row.get("location_symbol")
            })
            .fetch_all(&pg_pool)
            .await?
    )
}

pub async fn persist_flight_plan(pg_pool: PgPool, user_id: &str, ship_id: &str, flight_plan: &responses::FlightPlan) -> anyhow::Result<()> {
    sqlx::query("
        INSERT INTO daemon_flight_plan (
             id
            ,user_id
            ,ship_id
            ,origin
            ,destination
            ,distance
            ,fuel_consumed
            ,fuel_remaining
            ,time_remaining_in_seconds
            ,arrives_at
        ) VALUES ($1, $2::uuid, $3, $4, $5, $6, $7, $8, $9, $10);
    ")
        .bind(&flight_plan.flight_plan.id)
        .bind(&user_id)
        .bind(&ship_id)
        .bind(&flight_plan.flight_plan.departure)
        .bind(&flight_plan.flight_plan.destination)
        .bind(&flight_plan.flight_plan.distance)
        .bind(&flight_plan.flight_plan.fuel_consumed)
        .bind(&flight_plan.flight_plan.fuel_remaining)
        .bind(&flight_plan.flight_plan.time_remaining_in_seconds)
        .bind(&flight_plan.flight_plan.arrives_at)
        .execute(&pg_pool)
        .await?;

    Ok(())
}

pub async fn get_system_location(pg_pool: PgPool, location_symbol: String) -> anyhow::Result<DbSystemLocation> {
    Ok(
        sqlx::query("
            SELECT
                 system_symbol
                ,system_name
                ,location_symbol
                ,location_name
                ,location_type
                ,x
                ,y
                ,created_at
            FROM daemon_system_info
            WHERE location_symbol = $1;
        ")
            .bind(&location_symbol)
            .map(|row: PgRow| {
                DbSystemLocation {
                    system_symbol: row.get("system_symbol"),
                    system_name: row.get("system_name"),
                    location_symbol: row.get("location_symbol"),
                    location_name: row.get("location_name"),
                    location_type: row.get("location_type"),
                    x: row.get("x"),
                    y: row.get("y"),
                    created_at: row.get("created_at"),
                }
            })
            .fetch_one(&pg_pool)
            .await?
    )
}

pub async fn get_distance_between_locations(pg_pool: PgPool, origin: &str, destination: &str) -> anyhow::Result<DbDistanceBetweenLocations> {
    Ok(
        sqlx::query("
            SELECT
                 dsi1.location_type as origin_location_type
                ,SQRT(POW(dsi1.x - dsi2.x, 2) + POW(dsi1.y - dsi2.y, 2)) AS distance
            FROM daemon_system_info dsi1
            INNER JOIN daemon_system_info dsi2
                -- for now we are going to restrict this to the same system since we don't have
                -- multiple stops built yet
                ON dsi1.system_symbol = dsi2.system_symbol
            WHERE dsi1.location_symbol = $1
                AND dsi2.location_symbol = $2;
        ")
            .bind(origin)
            .bind(destination)
            .map(|row: PgRow| {
                DbDistanceBetweenLocations {
                    origin_location_type: row.get("origin_location_type"),
                    distance: row.get("distance"),
                }
            })
            .fetch_one(&pg_pool)
            .await?
    )
}

pub async fn get_active_flight_plan(pg_pool: PgPool, ship_id: &str) -> anyhow::Result<Option<shared::FlightPlanData>> {
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
            FROM daemon_flight_plan
            WHERE ship_id = $1
                AND arrives_at > $2
        ")
            .bind(ship_id)
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

pub async fn persist_market_data(pg_pool: PgPool, location_symbol: &str, marketplace_data: &shared::MarketplaceData) -> anyhow::Result<()> {
    sqlx::query("
        INSERT INTO daemon_market_data(location_symbol, good_symbol, price_per_unit, volume_per_unit, quantity_available, purchase_price_per_unit, sell_price_per_unit)
        VALUES ($1, $2, $3, $4, $5, $6, $7);
    ")
        .bind(location_symbol)
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

pub async fn get_routes_from_location(pg_pool: PgPool, location_symbol: &str, ship_speed: i32) -> anyhow::Result<Vec<DbRoute>> {
    let mut transaction = pg_pool.begin().await.unwrap();

    sqlx::query("DROP TABLE IF EXISTS tmp_latest_location_goods;")
        .execute(&mut transaction)
        .await?;

    sqlx::query("
        CREATE TEMPORARY TABLE tmp_latest_location_goods (
             location_symbol VARCHAR(100) NOT NULL
            ,location_type VARCHAR(100) NOT NULL
            ,x INT NOT NULL
            ,y INT NOT NULL
            ,good_symbol VARCHAR(100) NOT NULL
            ,quantity_available INT NOT NULL
            ,price_per_unit INT NOT NULL
            ,volume_per_unit INT NOT NULL
            ,created_at TIMESTAMP WITH TIME ZONE NOT NULL
        );
    ")
        .execute(&mut transaction)
        .await?;

    sqlx::query("
        -- Get the latest market data from each good in each location
        WITH ranked_location_goods AS (
            SELECT
                 id
                ,ROW_NUMBER() OVER (
                    PARTITION BY location_symbol, good_symbol
                    ORDER BY created_at DESC
                ) AS rank
            FROM daemon_market_data
        )
        INSERT INTO tmp_latest_location_goods (
             location_symbol
            ,location_type
            ,x
            ,y
            ,good_symbol
            ,quantity_available
            ,price_per_unit
            ,volume_per_unit
            ,created_at
        )
        SELECT
             dmd.location_symbol
            ,dsi.location_type
            ,dsi.x
            ,dsi.y
            ,dmd.good_symbol
            ,dmd.quantity_available
            ,dmd.price_per_unit
            ,dmd.volume_per_unit
            ,dmd.created_at
        FROM daemon_market_data dmd
        INNER JOIN ranked_location_goods rlg ON dmd.id = rlg.id
        INNER JOIN daemon_system_info dsi on dmd.location_symbol = dsi.location_symbol
        WHERE rlg.rank = 1
            AND dmd.created_at > (now() at time zone 'utc' - INTERVAL '30 min')
        ORDER BY dmd.good_symbol, dmd.location_symbol;
    ")
        .execute(&mut transaction)
        .await?;

    let mut routes = sqlx::query("
        -- calculate the route from each location to each location per good
        -- limited to routes which will actually turn a profit
        SELECT
             llg1.location_symbol AS purchase_location_symbol
            ,llg1.location_type AS purchase_location_type
            ,llg2.location_symbol AS sell_location_symbol
            ,llg2.good_symbol
            ,SQRT(POW(llg1.x - llg2.x, 2) + POW(llg2.y - llg1.y, 2)) AS distance
            ,llg1.quantity_available AS purchase_quantity
            ,llg2.quantity_available AS sell_quantity
            ,llg1.price_per_unit AS purchase_price_per_unit
            ,llg2.price_per_unit AS sell_price_per_unit
            ,llg1.volume_per_unit AS volume_per_unit
        FROM tmp_latest_location_goods llg1
        CROSS JOIN tmp_latest_location_goods llg2
        INNER JOIN daemon_system_info from_dsi
            ON from_dsi.location_symbol = llg1.location_symbol
        INNER JOIN daemon_system_info to_dsi
            ON to_dsi.location_symbol = llg2.location_symbol
        WHERE from_dsi.location_symbol = $1
            AND from_dsi.system_symbol = to_dsi.system_symbol
            AND llg1.good_symbol = llg2.good_symbol
            AND llg1.location_symbol != llg2.location_symbol
    ")
        .bind(location_symbol)
        .map(|row: PgRow| {
            let distance: f64 = row.get("distance");
            let location_type: String = row.get("purchase_location_type");
            let purchase_price_per_unit: i32 = row.get("purchase_price_per_unit");
            let sell_price_per_unit: i32 = row.get("sell_price_per_unit");
            let volume_per_unit: i32 = row.get("volume_per_unit");

            let planet_penalty = if location_type == "Planet" { 2.0 } else { 0.0 };
            let fuel_required: f64 = (distance.round() / 4.0).round() + planet_penalty + 1.0;

            let flight_time = (distance * (2.0 / f64::from(ship_speed)).round()) + 60.0;

            let profit = f64::from(sell_price_per_unit - purchase_price_per_unit);
            let cost_volume_distance = profit / f64::from(volume_per_unit) / distance;
            let profit_speed_volume_distance = (profit * f64::from(ship_speed)) / (f64::from(volume_per_unit) * distance);

            DbRoute {
                purchase_location_symbol: row.get("purchase_location_symbol"),
                purchase_location_type: row.get("purchase_location_type"),
                sell_location_symbol: row.get("sell_location_symbol"),
                good: Good::from(row.get::<String, &str>("good_symbol")),
                distance: row.get("distance"),
                purchase_quantity: row.get("purchase_quantity"),
                sell_quantity: row.get("sell_quantity"),
                purchase_price_per_unit,
                sell_price_per_unit,
                volume_per_unit,
                fuel_required,
                flight_time,
                cost_volume_distance,
                profit_speed_volume_distance,
            }
        })
        .fetch_all(&mut transaction)
        .await?;

    routes.sort_by(|a, b|
        b.profit_speed_volume_distance.partial_cmp(&a.profit_speed_volume_distance).unwrap_or(Equal)
    );

    Ok(routes)
}

pub async fn persist_user_stats(pg_pool: PgPool, user_id: &str, credits: i32, ships: &Vec<shared::Ship>) -> anyhow::Result<()> {
    sqlx::query("
        INSERT INTO daemon_user_stats (user_id, credits, ship_count, ships) VALUES ($1::uuid, $2, $3, $4::json);
    ")
        .bind(user_id)
        .bind(credits)
        .bind(ships.len() as i32)
        .bind(serde_json::to_string(&ships).unwrap())
        .execute(&pg_pool)
        .await?;

    Ok(())
}

#[derive(Serialize)]
struct DbRequest<'a> {
    method: &'a str,
    url: &'a str,
    body: Option<&'a str>,
}

#[derive(Serialize)]
struct DbResponse<'a> {
    status_code: Option<u16>,
    headers: Option<&'a HashMap<String, String>>,
    body: Option<&'a str>,
}

pub async fn persist_request_response(
    pg_pool: PgPool,
    method: &str, url: &str, request_body: Option<&str>,
    response_status_code: Option<u16>,
    response_headers: Option<&HashMap<String, String>>,
    response_body: Option<&str>,
    error: Option<&SpaceTradersClientError>,
) -> anyhow::Result<()> {
    let db_request = DbRequest {
        method,
        url,
        body: request_body,
    };

    let db_response = DbResponse {
        status_code: response_status_code,
        headers: response_headers,
        body: response_body,
    };

    let mut db_error = None;
    if let Some(error) = error {
        db_error = Some(format!("Error: {}", error));
    }

    sqlx::query("
        INSERT INTO http_log (request, response, error) VALUES ($1, $2, $3);
    ")
        .bind(serde_json::to_string(&db_request).unwrap())
        .bind(serde_json::to_string(&db_response).unwrap())
        .bind(db_error)
        .execute(&pg_pool)
        .await?;

    Ok(())
}

pub async fn persist_ship(pg_pool: PgPool, user_id: &str, ship: &shared::Ship) -> anyhow::Result<()> {
    sqlx::query("
        INSERT INTO daemon_user_ship (
             user_id
            ,ship_id
            ,type
            ,class
            ,max_cargo
            ,speed
            ,manufacturer
            ,plating
            ,weapons
        ) VALUES (
             $1::uuid
            ,$2
            ,$3
            ,$4
            ,$5
            ,$6
            ,$7
            ,$8
            ,$9
        )
        ON CONFLICT (user_id, ship_id)
        DO UPDATE SET
             type = $3
            ,class = $4
            ,max_cargo = $5
            ,speed = $6
            ,manufacturer = $7
            ,plating = $8
            ,weapons = $9
            ,modified_at = timezone('utc', NOW());
    ")
        .bind(user_id)
        .bind(&ship.id)
        .bind(&ship.ship_type)
        .bind(&ship.class)
        .bind(&ship.max_cargo)
        .bind(&ship.speed)
        .bind(&ship.manufacturer)
        .bind(&ship.plating)
        .bind(&ship.weapons)
        .execute(&pg_pool)
        .await?;

    Ok(())
}

pub async fn persist_transaction(pg_pool: PgPool, transaction_type: &str, user_id: &str, order: &responses::PurchaseOrder) -> anyhow::Result<()> {
    sqlx::query("
        INSERT INTO daemon_user_transaction (
             user_id
            ,ship_id
            ,type
            ,good_symbol
            ,price_per_unit
            ,quantity
            ,total
            ,location_symbol
        ) VALUES (
             $1::uuid
            ,$2
            ,$3
            ,$4
            ,$5
            ,$6
            ,$7
            ,$8
        )
    ")
        .bind(user_id)
        .bind(&order.ship.id)
        .bind(transaction_type)
        .bind(&order.order.good.to_string())
        .bind(&order.order.price_per_unit)
        .bind(&order.order.quantity)
        .bind(&order.order.total)
        .bind(&order.ship.location.clone().unwrap_or_else(|| "UNKNOWN".to_string()))
        .execute(&pg_pool)
        .await?;

    Ok(())
}

pub async fn get_fuel_required(pg_pool: PgPool, origin: &str, destination: &str, ship_type: &str) -> anyhow::Result<Option<i32>> {
    Ok(sqlx::query("
        SELECT fuel_consumed
        FROM daemon_flight_plan dfp
        INNER JOIN daemon_user_ship dus
            ON dus.ship_id = dfp.ship_id
        WHERE dfp.origin = $1
            AND dfp.destination = $2
            AND dus.type = $3
        LIMIT 1
    ")
        .bind(origin)
        .bind(destination)
        .bind(ship_type)
        .map(|row: PgRow| {
            row.get("fuel_consumed")
        })
        .fetch_optional(&pg_pool)
        .await?
    )
}
