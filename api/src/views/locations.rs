use actix_web::{web, get, Responder, HttpResponse};
use serde::Deserialize;
use sqlx::{PgPool, Error, Row};
use sqlx::postgres::PgRow;
use crate::models::MarketData;

#[get("/locations/{location_symbol}/goods")]
pub async fn goods(location_symbol: web::Path<String>, pg_pool: web::Data<PgPool>) -> impl Responder {
    let market_data_goods: Result<Vec<String>, Error> = sqlx::query("
        SELECT DISTINCT
            md.good_symbol
        FROM daemon_market_data md
        WHERE md.location_symbol = $1;
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
             md.id
            ,md.location_symbol
            ,si.system_symbol
            ,md.good_symbol
            ,md.price_per_unit
            ,md.volume_per_unit
            ,md.quantity_available
            ,md.created_at
            ,md.purchase_price_per_unit
            ,md.sell_price_per_unit
        FROM daemon_market_data md
        INNER JOIN daemon_system_info si ON si.location_symbol = md.location_symbol
        WHERE md.location_symbol = $1
        ORDER BY md.location_symbol, md.good_symbol, md.created_at;
    ")
        .bind(location_symbol.as_str())
        .map(|row: PgRow| {
            MarketData {
                id: row.get("id"),
                location_symbol: row.get("location_symbol"),
                system_symbol: row.get("system_symbol"),
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

#[derive(Deserialize)]
pub struct MarketDataQuery {
    days_ago: Option<i32>,
}

#[get("/locations/{location_symbol}/market-data/{good_symbol}")]
pub async fn goods_market_data(params: web::Path<(String, String)>, web::Query(info): web::Query<MarketDataQuery>, pg_pool: web::Data<PgPool>) -> impl Responder {
    let (location_symbol, good_symbol) = params.into_inner();
    let days_ago = if let Some(days_ago) = info.days_ago {
        days_ago
    } else {
        7
    };

    let days_ago = match days_ago {
        days_ago if days_ago > 30 => 30,
        days_ago if days_ago < 1 => 1,
        _ => days_ago,
    };

    let market_data_goods = sqlx::query("
        SELECT
             md.id
            ,md.location_symbol
            ,si.system_symbol
            ,md.good_symbol
            ,md.price_per_unit
            ,md.volume_per_unit
            ,md.quantity_available
            ,md.created_at
            ,md.purchase_price_per_unit
            ,md.sell_price_per_unit
        FROM daemon_market_data md
        INNER JOIN daemon_system_info si ON md.location_symbol = si.location_symbol
        WHERE md.location_symbol = $1
            AND md.good_symbol = $2
            AND md.created_at > date_trunc('day', NOW()) - ($3 || ' DAYS')::INTERVAL
        ORDER BY md.created_at DESC;
    ")
        .bind(location_symbol.to_owned())
        .bind(good_symbol.to_owned())
        .bind(days_ago.to_owned())
        .map(|row: PgRow| {
            MarketData {
                id: row.get("id"),
                location_symbol: row.get("location_symbol"),
                system_symbol: row.get("system_symbol"),
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
