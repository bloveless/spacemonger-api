use actix_web::{web, get, Responder, HttpResponse};
use sqlx::{PgPool, Error, Row};
use sqlx::postgres::PgRow;
use crate::models::MarketData;

#[get("/locations/{location_symbol}/goods")]
pub async fn goods(location_symbol: web::Path<String>, pg_pool: web::Data<PgPool>) -> impl Responder {
    let market_data_goods: Result<Vec<String>, Error> = sqlx::query("
        SELECT DISTINCT
            good_symbol
        FROM market_data
        WHERE location_symbol = $1;
    ")
        .bind(location_symbol.as_str())
        .map(|row: PgRow| {
            row.get("good_symbol")
        })
        .fetch_all(pg_pool.as_ref())
        .await;

    match market_data_goods {
        Ok(market_data_goods) => HttpResponse::Ok().json(market_data_goods),
        Err(e) => HttpResponse::InternalServerError().body(format!("Something went wrong: {:?}", e)),
    }
}

#[get("/locations/{location_symbol}/market-data")]
pub async fn market_data(location_symbol: web::Path<String>, pg_pool: web::Data<PgPool>) -> impl Responder {
    let market_data = sqlx::query("
        SELECT
             id
            ,location_symbol
            ,good_symbol
            ,price_per_unit
            ,volume_per_unit
            ,quantity_available
            ,created_at
            ,purchase_price_per_unit
            ,sell_price_per_unit
        FROM daemon.market_data
        WHERE location_symbol = $1
        ORDER BY location_symbol, good_symbol, created_at;
    ")
        .bind(location_symbol.as_str())
        .map(|row: PgRow| {
            MarketData {
                id: row.get("id"),
                location_symbol: row.get("location_symbol"),
                good_symbol: row.get("good_symbol"),
                price_per_unit: row.get("price_per_unit"),
                volume_per_unit: row.get("volume_per_unit"),
                quantity_available: row.get("quantity_available"),
                created_at: row.get("created_at"),
                purchase_price_per_unit: row.get("purchase_price_per_unit"),
                sell_price_per_unit: row.get("sell_price_per_unit"),
            }
        })
        .fetch_all(pg_pool.as_ref())
        .await;

    match market_data {
        Ok(results) => HttpResponse::Ok().json(results),
        Err(e) => HttpResponse::InternalServerError().body(format!("Something went wrong: {:?}", e)),
    }
}

#[get("/locations/{location_symbol}/market-data/{good_symbol}")]
pub async fn goods_market_data(params: web::Path<(String, String)>, pg_pool: web::Data<PgPool>) -> impl Responder {
    let (location_symbol, good_symbol) = params.into_inner();

    let market_data_goods = sqlx::query("
        SELECT
             id
            ,location_symbol
            ,good_symbol
            ,price_per_unit
            ,volume_per_unit
            ,quantity_available
            ,created_at
            ,purchase_price_per_unit
            ,sell_price_per_unit
        FROM market_data
        WHERE location_symbol = $1
            AND good_symbol = $2
        ORDER BY created_at;
    ")
        .bind(location_symbol.as_str())
        .bind(good_symbol.as_str())
        .map(|row: PgRow| {
            MarketData {
                id: row.get("id"),
                location_symbol: row.get("location_symbol"),
                good_symbol: row.get("good_symbol"),
                price_per_unit: row.get("price_per_unit"),
                volume_per_unit: row.get("volume_per_unit"),
                quantity_available: row.get("quantity_available"),
                created_at: row.get("created_at"),
                purchase_price_per_unit: row.get("purchase_price_per_unit"),
                sell_price_per_unit: row.get("sell_price_per_unit"),
            }
        })
        .fetch_all(pg_pool.as_ref())
        .await;

    match market_data_goods {
        Ok(market_data_goods) => HttpResponse::Ok().json(market_data_goods),
        Err(e) => HttpResponse::InternalServerError().body(format!("Something went wrong: {:?}", e)),
    }
}
