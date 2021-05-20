use actix_web::{web, HttpResponse, Responder, get};
use sqlx::{PgPool, Row};
use sqlx::postgres::PgRow;
use crate::models::User;

#[get("/users")]
pub async fn users(pg_pool: web::Data<PgPool>) -> impl Responder {
    let results = sqlx::query("
            SELECT
                 u.id::text
                ,u.username
                ,u.token
                ,u.assignment
                ,u.system_symbol
                ,u.location_symbol
            FROM daemon_user u
            LIMIT 1;
        ")
        .map(|row: PgRow| {
            User {
                id: row.get("id"),
                username: row.get("username"),
                token: row.get("token"),
                assignment: row.get("assignment"),
                system_symbol: row.get("system_symbol"),
                location_symbol: row.get("location_symbol"),
            }
        })
        .fetch_all(pg_pool.get_ref())
        .await;

    match results {
        Ok(results) => HttpResponse::Ok().json(results),
        _ => HttpResponse::BadRequest().body("Error trying to do something"),
    }
}
