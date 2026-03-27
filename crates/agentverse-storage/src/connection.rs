use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use sea_orm::{
    ConnectOptions, ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, ExecResult,
    QueryResult, Statement,
};

/// A cheaply cloneable handle to the shared database connection pool.
///
/// `DatabaseConnection` is not `Clone` in sea-orm 1.x (the underlying sqlx
/// pool is ref-counted but Clone is not derived).  Wrapping it in `Arc` lets
/// every repository and service share the *same* pool without copying.
///
/// Implements `Deref<Target = DatabaseConnection>` so all sea-orm query
/// helpers (`.one(&self.db)`, `.all(&self.db)`, etc.) work transparently
/// via auto-deref.
#[derive(Clone)]
pub struct DatabasePool(Arc<DatabaseConnection>);

impl DatabasePool {
    /// Wrap an existing `DatabaseConnection` — useful in tests.
    pub fn from_connection(conn: DatabaseConnection) -> Self {
        Self(Arc::new(conn))
    }
}

impl std::ops::Deref for DatabasePool {
    type Target = DatabaseConnection;
    fn deref(&self) -> &DatabaseConnection {
        &self.0
    }
}

/// Delegate `ConnectionTrait` to the inner `DatabaseConnection` so that
/// all sea-orm query helpers work transparently with `DatabasePool`.
#[async_trait]
impl ConnectionTrait for DatabasePool {
    fn get_database_backend(&self) -> DatabaseBackend {
        self.0.get_database_backend()
    }

    async fn execute(&self, stmt: Statement) -> Result<ExecResult, DbErr> {
        self.0.execute(stmt).await
    }

    async fn execute_unprepared(&self, sql: &str) -> Result<ExecResult, DbErr> {
        self.0.execute_unprepared(sql).await
    }

    async fn query_one(&self, stmt: Statement) -> Result<Option<QueryResult>, DbErr> {
        self.0.query_one(stmt).await
    }

    async fn query_all(&self, stmt: Statement) -> Result<Vec<QueryResult>, DbErr> {
        self.0.query_all(stmt).await
    }

    fn support_returning(&self) -> bool {
        self.0.support_returning()
    }

    fn is_mock_connection(&self) -> bool {
        self.0.is_mock_connection()
    }
}

/// Database factory — call `Database::connect()` once at startup.
pub struct Database;

impl Database {
    /// Connect to PostgreSQL and return a cloneable pool handle.
    pub async fn connect(url: &str) -> Result<DatabasePool, sea_orm::DbErr> {
        let mut opts = ConnectOptions::new(url.to_string());
        opts.max_connections(20)
            .min_connections(2)
            .connect_timeout(Duration::from_secs(10))
            .acquire_timeout(Duration::from_secs(10))
            .idle_timeout(Duration::from_secs(300))
            .sqlx_logging(false);

        sea_orm::Database::connect(opts)
            .await
            .map(DatabasePool::from_connection)
    }
}
