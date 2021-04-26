use actix_web::{Responder, web, get, HttpResponse};
use sqlx::{PgPool, Row, Error};
use sqlx::postgres::PgRow;
use crate::models::{SystemInfo, Route};

#[get("/systems")]
pub async fn info(pg_pool: web::Data<PgPool>) -> impl Responder {
    let systems = sqlx::query("
        SELECT
             si.system_symbol
            ,si.system_name
            ,si.location_symbol
            ,si.location_name
            ,si.location_type
            ,si.x
            ,si.y
            ,si.created_at
        FROM daemon_system_info si;
    ")
        .map(|row: PgRow| {
            SystemInfo {
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
        .fetch_all(pg_pool.as_ref())
        .await;

    match systems {
        Ok(results) => HttpResponse::Ok().json(results),
        Err(e) => HttpResponse::InternalServerError().body(format!("Something went wrong: {:?}", e)),
    }
}

#[get("/systems/{system_symbol}/goods")]
pub async fn goods(system_symbol: web::Path<String>, pg_pool: web::Data<PgPool>) -> impl Responder {
    let system_goods: Result<Vec<String>, Error> = sqlx::query("
        SELECT DISTINCT good_symbol FROM daemon_market_data dmd
        INNER JOIN daemon_system_info dsi
            ON dmd.location_symbol = dsi.location_symbol
        WHERE dsi.system_symbol = 'OE'
    ")
        .bind(system_symbol.to_owned())
        .map(|row: PgRow| {
            row.get("good_symbol")
        })
        .fetch_all(pg_pool.as_ref())
        .await;

    match system_goods {
        Ok(results) => HttpResponse::Ok().json(results),
        Err(e) => HttpResponse::InternalServerError().body(format!("Something went wrong: {:?}", e)),
    }
}

#[get("/systems/{system_symbol}/routes/{good_symbol}")]
pub async fn routes(params: web::Path<(String, String)>, pg_pool: web::Data<PgPool>) -> impl Responder {
    let (system_symbol, good_symbol) = params.into_inner();

    let mut tx = pg_pool.begin().await.unwrap();

    sqlx::query("DROP TABLE IF EXISTS tmp_latest_location_goods;")
        .execute(&mut tx)
        .await.unwrap();

    sqlx::query("
        CREATE TEMPORARY TABLE tmp_latest_location_goods (
             location_symbol VARCHAR(100) NOT NULL
            ,x INT NOT NULL
            ,y INT NOT NULL
            ,good_symbol VARCHAR(100) NOT NULL
            ,quantity_available INT NOT NULL
            ,purchase_price_per_unit INT NOT NULL
            ,sell_price_per_unit INT NOT NULL
            ,created_at TIMESTAMP WITH TIME ZONE NOT NULL
        );
    ")
        .execute(&mut tx)
        .await.unwrap();


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
            ,x
            ,y
            ,good_symbol
            ,quantity_available
            ,purchase_price_per_unit
            ,sell_price_per_unit
            ,created_at
        )
        SELECT
             dmd.location_symbol
            ,dsi.x
            ,dsi.y
            ,dmd.good_symbol
            ,dmd.quantity_available
            ,dmd.purchase_price_per_unit
            ,dmd.sell_price_per_unit
            ,dmd.created_at
        FROM daemon_market_data dmd
        INNER JOIN ranked_location_goods rlg ON dmd.id= rlg.id
        INNER JOIN daemon_system_info dsi on dmd.location_symbol = dsi.location_symbol
        WHERE rlg.rank = 1
            AND dsi.system_symbol = $1
            AND dmd.good_symbol = $2
        ORDER BY dmd.good_symbol, dmd.location_symbol;
    ")
        .bind(system_symbol.to_owned())
        .bind(good_symbol.to_owned())
        .execute(&mut tx)
        .await.unwrap();

    let system_routes = sqlx::query("
        -- calculate the route from each location to each location per good
        SELECT
             t.purchase_location_symbol
            ,t.sell_location_symbol
            ,t.good_symbol
            ,t.purchase_x
            ,t.purchase_y
            ,t.sell_x
            ,t.sell_y
            ,t.distance
            ,purchase_dsi.location_type AS purchase_location_type
            ,CASE
                WHEN purchase_dsi.location_type = 'Planet' THEN CEIL((t.distance / 4) + 2 + 1)::INT
                ELSE CEIL((t.distance / 4) + 1)::INT
             END AS approximate_fuel
            ,t.purchase_quantity_available
            ,t.sell_quantity_available
            ,t.purchase_price_per_unit
            ,t.sell_price_per_unit
            ,t.purchase_created_at
            ,t.sell_created_at
        FROM (
            SELECT
                 llg1.location_symbol AS purchase_location_symbol
                ,llg2.location_symbol AS sell_location_symbol
                ,llg2.good_symbol
                ,llg1.x AS purchase_x
                ,llg1.y AS purchase_y
                ,llg2.x AS sell_x
                ,llg2.y AS sell_y
                ,SQRT(POW(llg1.x - llg2.x, 2) + POW(llg2.y - llg1.y, 2)) AS distance
                ,llg1.quantity_available AS purchase_quantity_available
                ,llg2.quantity_available AS sell_quantity_available
                ,llg1.purchase_price_per_unit AS purchase_price_per_unit
                ,llg2.sell_price_per_unit AS sell_price_per_unit
                ,llg1.created_at AS purchase_created_at
                ,llg2.created_at AS sell_created_at
            FROM tmp_latest_location_goods llg1
            CROSS JOIN tmp_latest_location_goods llg2
            WHERE llg1.good_symbol = llg2.good_symbol
                AND llg1.location_symbol != llg2.location_symbol
        ) as t
        INNER JOIN daemon_system_info purchase_dsi
            ON purchase_dsi.location_symbol = t.purchase_location_symbol;
    ")
        .map(|row: PgRow| {
            Route {
                purchase_location_symbol: row.get("purchase_location_symbol"),
                sell_location_symbol: row.get("sell_location_symbol"),
                good_symbol: row.get("good_symbol"),
                purchase_x: row.get("purchase_x"),
                purchase_y: row.get("purchase_y"),
                sell_x: row.get("sell_x"),
                sell_y: row.get("sell_y"),
                distance: row.get("distance"),
                purchase_location_type: row.get("purchase_location_type"),
                approximate_fuel: row.get("approximate_fuel"),
                purchase_quantity_available: row.get("purchase_quantity_available"),
                sell_quantity_available: row.get("sell_quantity_available"),
                purchase_price_per_unit: row.get("purchase_price_per_unit"),
                sell_price_per_unit: row.get("sell_price_per_unit"),
                purchase_created_at: row.get("purchase_created_at"),
                sell_created_at: row.get("sell_created_at"),
            }
        })
        .fetch_all(&mut tx)
        .await;

    match system_routes {
        Ok(system_routes) => HttpResponse::Ok().json(system_routes),
        Err(e) => HttpResponse::InternalServerError().body(format!("Something went wrong: {:?}", e)),
    }
}
