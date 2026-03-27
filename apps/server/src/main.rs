//! agentverse server binary entry point.

use std::{net::SocketAddr, sync::Arc};

use axum::routing::post;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use agentverse_api::{
    mcp::mcp_handler,
    routes::build_router,
    state::{AppConfig, AppState},
};
use agentverse_auth::JwtManager;
use agentverse_events::EventStore;
use agentverse_search::{FullTextSearch, SemanticSearch};
use agentverse_storage::{ArtifactRepo, Database, SocialRepo, UserRepo, VersionRepo};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env if present
    let _ = dotenvy::dotenv();

    // Init structured logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://agentverse:agentverse_dev@localhost:5432/agentverse".into()
    });
    let jwt_secret =
        std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-in-production".into());

    // Connect to PostgreSQL
    tracing::info!("connecting to database…");
    let db = Database::connect(&database_url)
        .await
        .map_err(|e| anyhow::anyhow!("DB connect failed: {e}"))?;
    tracing::info!("database connected");

    // Run pending SQL migrations (embedded at compile time from ../../migrations)
    tracing::info!("running database migrations…");
    sqlx::migrate!("../../migrations")
        .run(db.get_postgres_connection_pool())
        .await
        .map_err(|e| anyhow::anyhow!("migration failed: {e}"))?;
    tracing::info!("migrations applied");

    let config = AppConfig {
        jwt_secret: jwt_secret.clone(),
        anonymous_read: true,
        auto_infer_bump: true,
        access_token_expiry_secs: 86_400,
    };

    let state = AppState {
        artifacts: Arc::new(ArtifactRepo::new(db.clone())),
        versions: Arc::new(VersionRepo::new(db.clone())),
        social: Arc::new(SocialRepo::new(db.clone())),
        users: Arc::new(UserRepo::new(db.clone())),
        events: Arc::new(EventStore::new(db.clone())),
        fulltext: Arc::new(FullTextSearch::new(db.clone())),
        semantic: Arc::new(SemanticSearch::new(db.clone())),
        jwt: Arc::new(JwtManager::new(
            &jwt_secret,
            config.access_token_expiry_secs,
        )),
        config: Arc::new(config),
    };

    let app = build_router(state.clone())
        .route("/mcp", post(mcp_handler))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("agentverse listening on http://{addr}");
    tracing::info!("MCP endpoint: http://{addr}/mcp");
    tracing::info!("Swagger UI:   http://{addr}/swagger-ui/");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

// Real repositories are injected in main(). Integration tests should start the server
// and connect via HTTP, or use agentverse-storage directly with a test database.
