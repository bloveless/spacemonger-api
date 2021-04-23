mod db;
mod views;
mod models;

use actix_web::{web, App, HttpServer, middleware};
use actix_web::middleware::Logger;
use anyhow::Result;
use std::env;
use dotenv::dotenv;
use actix_cors::Cors;

#[actix_web::main]
async fn main() -> Result<()> {
    dotenv().ok();
    std::env::set_var("RUST_LOG", "actix_web=info,actix=info");
    env_logger::init();

    let postgres_host = env::var("POSTGRES_HOST").unwrap();
    let postgres_port = env::var("POSTGRES_PORT").unwrap().parse::<i32>().unwrap();
    let postgres_username = env::var("POSTGRES_USERNAME").unwrap();
    let postgres_password = env::var("POSTGRES_PASSWORD").unwrap();
    let postgres_database = env::var("POSTGRES_DATABASE").unwrap();

    let pg_pool = db::get_db_pool(postgres_host, postgres_port, postgres_username, postgres_password, postgres_database).await?;
    // db::run_migrations(pg_pool.clone()).await?;

    HttpServer::new(move || App::new()
        .wrap(Cors::permissive())
        .wrap(middleware::NormalizePath::default())
        .wrap(Logger::default())
        .service(web::scope("/api/").configure(views::init))
        .app_data(web::Data::new(pg_pool.clone()))
    )
        .bind("0.0.0.0:8080")?
        .run()
        .await?;

    Ok(())
}
