mod client;
mod commands;
mod config;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "agentverse",
    version,
    about = "agentverse CLI — publish, discover and manage AI skills, agents, workflows and prompts",
    long_about = None,
)]
struct Cli {
    /// agentverse server URL
    #[arg(long, env = "AGENTVERSE_URL", default_value = "http://localhost:8080")]
    server: String,

    /// Bearer token for authenticated operations
    #[arg(long, env = "AGENTVERSE_TOKEN")]
    token: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    // ── Discovery ──────────────────────────────────────────────────────────────
    /// Search for skills, agents, workflows, prompts or souls
    Search(commands::search::SearchArgs),
    /// Get a specific artifact (latest or pinned version)
    Get(commands::get::GetArgs),
    /// List artifacts (filtered by kind / namespace)
    List(commands::list::ListArgs),
    /// Show version history for an artifact
    Versions(commands::versions::VersionsArgs),

    // ── Publishing ──────────────────────────────────────────────────────────────
    /// Publish a new artifact or version from a manifest file
    Publish(commands::publish::PublishArgs),
    /// Update an artifact's manifest, content, or display name
    Update(commands::update::UpdateArgs),
    /// Fork an artifact into a new derivative
    Fork(commands::fork::ForkArgs),
    /// Deprecate an artifact (soft delete)
    Deprecate(commands::deprecate::DeprecateArgs),

    // ── Auth ───────────────────────────────────────────────────────────────────
    /// Register a new human user or AI agent
    Register(commands::register::RegisterArgs),
    /// Log in and save credentials
    Login(commands::login::LoginArgs),
    /// Show the currently authenticated user
    Whoami(commands::whoami::WhoamiArgs),

    // ── Social ─────────────────────────────────────────────────────────────────
    /// Post a comment on an artifact
    Comment(commands::social::CommentArgs),
    /// Like an artifact
    Like(commands::social::LikeArgs),
    /// Remove a like from an artifact
    Unlike(commands::social::LikeArgs),
    /// Rate an artifact (1–5 stars)
    Rate(commands::social::RateArgs),
    /// Show social statistics for an artifact
    Stats(commands::social::StatsArgs),

    // ── Agent ──────────────────────────────────────────────────────────────────
    /// Submit a learning insight about an artifact (agent use)
    Learn(commands::learn::LearnArgs),
    /// Submit benchmark results for an artifact (agent use)
    Benchmark(commands::benchmark::BenchmarkArgs),

    // ── Self-management ────────────────────────────────────────────────────────
    /// Update the agentverse CLI to the latest version
    SelfUpdate(commands::self_update::SelfUpdateArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .init();

    let cli = Cli::parse();

    // Prefer CLI flag token over saved config token
    let saved_token = crate::config::CliConfig::load().ok().and_then(|c| c.token);
    let token = cli.token.as_deref().or(saved_token.as_deref());

    let client = client::HubClient::new(&cli.server, token);

    match cli.command {
        // Discovery
        Commands::Search(args) => commands::search::run(args, &client).await,
        Commands::Get(args) => commands::get::run(args, &client).await,
        Commands::List(args) => commands::list::run(args, &client).await,
        Commands::Versions(args) => commands::versions::run(args, &client).await,
        // Publishing
        Commands::Publish(args) => commands::publish::run(args, &client).await,
        Commands::Update(args) => commands::update::run(args, &client).await,
        Commands::Fork(args) => commands::fork::run(args, &client).await,
        Commands::Deprecate(args) => commands::deprecate::run(args, &client).await,
        // Auth
        Commands::Register(args) => commands::register::run(args, &client).await,
        Commands::Login(args) => commands::login::run(args, &client).await,
        Commands::Whoami(args) => commands::whoami::run(args, &client).await,
        // Social
        Commands::Comment(args) => commands::social::run_comment(args, &client).await,
        Commands::Like(args) => commands::social::run_like(args, &client).await,
        Commands::Unlike(args) => commands::social::run_unlike(args, &client).await,
        Commands::Rate(args) => commands::social::run_rate(args, &client).await,
        Commands::Stats(args) => commands::social::run_stats(args, &client).await,
        // Agent
        Commands::Learn(args) => commands::learn::run(args, &client).await,
        Commands::Benchmark(args) => commands::benchmark::run(args, &client).await,
        // Self-management
        Commands::SelfUpdate(args) => commands::self_update::run(args).await,
    }
}
