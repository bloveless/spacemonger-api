use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn get_db_pool(host: String, port: i32, username: String, password: String, database: String) -> Result<PgPool, anyhow::Error> {
    let pg_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&format!("postgresql://{}:{}@{}:{}/{}", username, password, host, port, database))
        .await?;

    Ok(pg_pool)
}

pub async fn run_migrations(pg_pool: PgPool) -> Result<(), anyhow::Error> {
    sqlx::migrate!("./migrations")
        .run(&pg_pool)
        .await
        .expect("Failed to migrate database");

    Ok(())
}
