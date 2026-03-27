use sea_orm::{ConnectOptions, DatabaseConnection};
use std::time::Duration;

/// Type alias for the SeaORM connection pool.
pub type DatabasePool = DatabaseConnection;

/// Database connection manager.
pub struct Database;

impl Database {
    /// Connect to PostgreSQL using the provided URL.
    pub async fn connect(url: &str) -> Result<DatabasePool, sea_orm::DbErr> {
        let mut opts = ConnectOptions::new(url.to_string());
        opts.max_connections(20)
            .min_connections(2)
            .connect_timeout(Duration::from_secs(10))
            .acquire_timeout(Duration::from_secs(10))
            .idle_timeout(Duration::from_secs(300))
            .sqlx_logging(false);

        sea_orm::Database::connect(opts).await
    }
}

