use actix_web::{Responder, web, get, HttpResponse};
use sqlx::{PgPool, Row};
use sqlx::postgres::PgRow;
use crate::models::SystemInfo;

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
