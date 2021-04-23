use actix_web::{web, Responder, HttpResponse, get};
use sqlx::{PgPool, Row};
use sqlx::postgres::PgRow;
use crate::models::MarketData;

#[get("/market-data/latest")]
pub async fn latest(pg_pool: web::Data<PgPool>) -> impl Responder {
    let market_data_latest = sqlx::query("
        WITH ranked_location_goods AS (
            SELECT
                 md.id
                ,ROW_NUMBER() OVER (
                    PARTITION BY md.location_symbol, md.good_symbol
                    ORDER BY md.created_at DESC
                ) AS rank
            FROM daemon_market_data md
        )
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
        INNER JOIN ranked_location_goods rlg ON md.id = rlg.id
        INNER JOIN daemon_system_info si ON md.location_symbol = si.location_symbol
        WHERE rlg.rank = 1
        ORDER BY md.location_symbol;
    ")
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

    match market_data_latest {
        Ok(results) => HttpResponse::Ok().json(results),
        Err(e) => HttpResponse::InternalServerError().body(format!("Something went wrong: {:?}", e)),
    }
}
