//! agentverse server binary entry point.

use std::{net::SocketAddr, sync::Arc};

use axum::routing::post;
use clap::Parser;
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
use agentverse_storage::{
    build_object_store,
    object_store::config::{LocalConfig, ObjectStoreBackend, ObjectStoreConfig},
    ArtifactRepo, Database, SkillInstallRepo, SkillPackageRepo, SocialRepo, UserRepo, VersionRepo,
};

// ── CLI args ──────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "agentverse-server", version, about = "AgentVerse server")]
struct Cli {
    /// Update the server binary to the latest GitHub release and exit
    #[arg(long)]
    self_update: bool,

    /// Check whether a newer release exists without installing it, then exit
    #[arg(long)]
    check_update: bool,

    /// GitHub personal access token for release API (avoids rate limits)
    #[arg(long, env = "GITHUB_TOKEN")]
    token: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env if present
    let _ = dotenvy::dotenv();

    // Parse CLI args before logging so --self-update / --check-update can exit early
    let cli = Cli::parse();

    // Init structured logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // ── Self-update handling ──────────────────────────────────────────────────
    if cli.check_update || cli.self_update {
        let current = env!("CARGO_PKG_VERSION");
        tracing::info!("checking for updates (current: v{current})...");

        match agentverse_updater::check_for_update(
            current,
            "agentverse-server",
            cli.token.as_deref(),
        )
        .await?
        {
            None => {
                tracing::info!("already up to date (v{current})");
            }
            Some(info) => {
                tracing::info!("new version available: v{}", info.version);
                if cli.self_update {
                    tracing::info!("downloading {}...", info.asset_name);
                    agentverse_updater::apply_update(&info, cli.token.as_deref()).await?;
                    tracing::info!("updated to v{} — restart the service", info.version);
                } else {
                    tracing::info!(
                        "run `agentverse-server --self-update` to install v{}",
                        info.version
                    );
                }
            }
        }
        return Ok(());
    }

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

    // ── Object store (optional) ───────────────────────────────────────────────
    // Resolve object store backend from environment variables.
    // If OBJECT_STORE_BACKEND is not set, fall back to a temporary local store
    // so the server starts without any extra config (useful for dev / E2E tests).
    let object_store_cfg = build_object_store_config_from_env();
    let object_store = match object_store_cfg {
        Some(cfg) => match build_object_store(&cfg) {
            Ok(store) => {
                tracing::info!(backend = store.backend_name(), "object store ready");
                Some(store)
            }
            Err(e) => {
                tracing::warn!("object store init failed: {e} — uploads will return 501");
                None
            }
        },
        None => {
            tracing::info!("no OBJECT_STORE_BACKEND set; using temporary local store for dev");
            let tmp = std::env::temp_dir().join("agentverse-packages");
            let local_cfg = ObjectStoreConfig {
                backend: ObjectStoreBackend::Local(LocalConfig {
                    base_dir: tmp,
                    serve_url: format!("http://0.0.0.0:{port}/files"),
                }),
            };
            build_object_store(&local_cfg).ok()
        }
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
        skill_packages: Arc::new(SkillPackageRepo::new(db.clone())),
        skill_installs: Arc::new(SkillInstallRepo::new(db.clone())),
        object_store,
        // Production always uses the real GitHub raw-content host.
        github_raw_base_url: None,
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

// ── Object store config from environment ──────────────────────────────────────
//
// Environment variables (all optional):
//
//   OBJECT_STORE_BACKEND   = s3 | local | github | custom
//
//   # S3 / COS / MinIO / R2
//   OBJECT_STORE_S3_ENDPOINT          custom endpoint URL (empty = AWS S3)
//   OBJECT_STORE_S3_ACCESS_KEY
//   OBJECT_STORE_S3_SECRET_KEY
//   OBJECT_STORE_S3_BUCKET
//   OBJECT_STORE_S3_REGION            default: us-east-1
//   OBJECT_STORE_S3_PUBLIC_URL_BASE   CDN override (optional)
//   OBJECT_STORE_S3_FORCE_PATH_STYLE  "true" for MinIO
//   OBJECT_STORE_S3_PRESIGNED_EXPIRY  seconds; 0 = public URL
//
//   # Local disk
//   OBJECT_STORE_LOCAL_BASE_DIR       absolute path to storage directory
//   OBJECT_STORE_LOCAL_SERVE_URL      HTTP prefix served at /files/*
//
//   # Custom HTTP
//   OBJECT_STORE_CUSTOM_UPLOAD_URL
//   OBJECT_STORE_CUSTOM_DOWNLOAD_URL_BASE
//   OBJECT_STORE_CUSTOM_UPLOAD_AUTH_HEADER   (optional)
//   OBJECT_STORE_CUSTOM_DOWNLOAD_AUTH_TYPE   none | query_param | bearer_header
//   OBJECT_STORE_CUSTOM_DOWNLOAD_AUTH_PARAM  query param name (for query_param)
//   OBJECT_STORE_CUSTOM_DOWNLOAD_AUTH_TOKEN  token value
//
//   # GitHub Releases
//   OBJECT_STORE_GITHUB_OWNER
//   OBJECT_STORE_GITHUB_REPO
//   GITHUB_TOKEN                      reused from standard env var

fn build_object_store_config_from_env() -> Option<ObjectStoreConfig> {
    use agentverse_storage::object_store::config::*;

    let backend_str = std::env::var("OBJECT_STORE_BACKEND").ok()?;

    let backend = match backend_str.to_lowercase().as_str() {
        "s3" => {
            let endpoint = std::env::var("OBJECT_STORE_S3_ENDPOINT")
                .ok()
                .filter(|s| !s.is_empty());
            ObjectStoreBackend::S3(S3Config {
                endpoint,
                access_key: std::env::var("OBJECT_STORE_S3_ACCESS_KEY").unwrap_or_default(),
                secret_key: std::env::var("OBJECT_STORE_S3_SECRET_KEY").unwrap_or_default(),
                bucket: std::env::var("OBJECT_STORE_S3_BUCKET")
                    .unwrap_or_else(|_| "agentverse".into()),
                region: std::env::var("OBJECT_STORE_S3_REGION")
                    .unwrap_or_else(|_| "us-east-1".into()),
                public_url_base: std::env::var("OBJECT_STORE_S3_PUBLIC_URL_BASE")
                    .ok()
                    .filter(|s| !s.is_empty()),
                force_path_style: std::env::var("OBJECT_STORE_S3_FORCE_PATH_STYLE")
                    .map(|v| v == "true")
                    .unwrap_or(false),
                presigned_expiry_secs: std::env::var("OBJECT_STORE_S3_PRESIGNED_EXPIRY")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0),
            })
        }
        "local" => {
            let base_dir = std::env::var("OBJECT_STORE_LOCAL_BASE_DIR").unwrap_or_else(|_| {
                std::env::temp_dir()
                    .join("agentverse-packages")
                    .to_string_lossy()
                    .into_owned()
            });
            let serve_url = std::env::var("OBJECT_STORE_LOCAL_SERVE_URL")
                .unwrap_or_else(|_| "http://localhost:8080/files".into());
            ObjectStoreBackend::Local(LocalConfig {
                base_dir: base_dir.into(),
                serve_url,
            })
        }
        "github" => ObjectStoreBackend::GitHub(GitHubConfig {
            owner: std::env::var("OBJECT_STORE_GITHUB_OWNER").unwrap_or_default(),
            repo: std::env::var("OBJECT_STORE_GITHUB_REPO").unwrap_or_default(),
            token: std::env::var("GITHUB_TOKEN").ok(),
        }),
        "custom" => {
            let auth_type = std::env::var("OBJECT_STORE_CUSTOM_DOWNLOAD_AUTH_TYPE")
                .unwrap_or_else(|_| "none".into());
            let download_auth = match auth_type.as_str() {
                "query_param" => DownloadAuth::QueryParam {
                    param: std::env::var("OBJECT_STORE_CUSTOM_DOWNLOAD_AUTH_PARAM")
                        .unwrap_or_else(|_| "token".into()),
                    token: std::env::var("OBJECT_STORE_CUSTOM_DOWNLOAD_AUTH_TOKEN")
                        .unwrap_or_default(),
                },
                "bearer_header" => DownloadAuth::BearerHeader {
                    token: std::env::var("OBJECT_STORE_CUSTOM_DOWNLOAD_AUTH_TOKEN")
                        .unwrap_or_default(),
                },
                _ => DownloadAuth::None,
            };
            ObjectStoreBackend::Custom(CustomConfig {
                upload_url: std::env::var("OBJECT_STORE_CUSTOM_UPLOAD_URL").unwrap_or_default(),
                download_url_base: std::env::var("OBJECT_STORE_CUSTOM_DOWNLOAD_URL_BASE")
                    .unwrap_or_default(),
                upload_auth_header: std::env::var("OBJECT_STORE_CUSTOM_UPLOAD_AUTH_HEADER").ok(),
                download_auth,
            })
        }
        other => {
            tracing::warn!("unknown OBJECT_STORE_BACKEND={other}; ignoring");
            return None;
        }
    };

    Some(ObjectStoreConfig { backend })
}
