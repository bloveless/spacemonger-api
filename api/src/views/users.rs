use actix_web::{web, HttpResponse, Responder, get, HttpRequest};
use sqlx::{PgPool, Row, Error};
use sqlx::postgres::PgRow;
use crate::models::{User, UserStats, UserStatsResponse};

#[get("/users")]
pub async fn users(pg_pool: web::Data<PgPool>) -> impl Responder {
    let results = sqlx::query("
            ;WITH user_stats AS (
                SELECT
                     user_id
                    ,credits
                    ,ship_count
                    ,created_at
                    ,ROW_NUMBER() OVER (ORDER BY created_at DESC) as rank
                FROM daemon_user_stats
            )
            SELECT
                 u.id::text
                ,u.username
                ,u.assignment
                ,u.system_symbol
                ,u.location_symbol
                ,us.credits
                ,us.ship_count
                ,us.created_at as stats_updated_at
            FROM daemon_user u
            INNER JOIN user_stats us
                ON u.id = us.user_id
            WHERE us.rank = 1;
        ")
        .map(|row: PgRow| {
            User {
                id: row.get("id"),
                username: row.get("username"),
                assignment: row.get("assignment"),
                system_symbol: row.get("system_symbol"),
                location_symbol: row.get("location_symbol"),
                credits: row.get("credits"),
                ship_count: row.get("ship_count"),
                stats_updated_at: row.get("stats_updated_at"),
            }
        })
        .fetch_all(pg_pool.get_ref())
        .await;

    match results {
        Ok(results) => HttpResponse::Ok().json(results),
        _ => HttpResponse::BadRequest().body("Error trying to do something"),
    }
}

#[get("/users/{user_id}")]
pub async fn user_stats(user_id: web::Path<String>, pg_pool: web::Data<PgPool>) -> impl Responder {
    let username: Result<String, Error> = sqlx::query("
        SELECT
            u.username
        FROM daemon_user u
        WHERE u.id = $1::uuid
        LIMIT 1;
    ")
        .bind(user_id.as_str())
        .map(|row: PgRow| {
            row.get("username")
        })
        .fetch_one(pg_pool.get_ref())
        .await;

    if username.is_err() {
        return HttpResponse::InternalServerError().body("Unable to get username");
    }

    let user_stats = sqlx::query("
        SELECT
             user_id::text
            ,credits
            ,ship_count
            ,ships::text
            ,created_at
        FROM daemon_user_stats dus
        WHERE dus.user_id = $1::uuid
        ORDER BY created_at ASC;
    ")
        .bind(user_id.as_str())
        .map(|row: PgRow| {
            UserStats {
                user_id: row.get("user_id"),
                credits: row.get("credits"),
                ship_count: row.get("ship_count"),
                ships: row.get("ships"),
                created_at: row.get("created_at"),
            }
        })
        .fetch_all(pg_pool.as_ref())
        .await;

    match user_stats {
        Ok(user_stats) => {
            HttpResponse::Ok().json(UserStatsResponse {
                username: username.unwrap(),
                stats: user_stats,
            })
        },
        _ => HttpResponse::BadRequest().body("Error trying to get user stats"),
    }
}
