// Main application entrypoint. Sets up configuration, logging, DI wiring and starts the Axum server.
// Showcases ownership of state via Arc, and graceful shutdown with Tokio.

use std::{net::SocketAddr, sync::Arc};

use axum::Router;
use tower_http::{cors::{Any, CorsLayer}, compression::CompressionLayer, trace::TraceLayer};
use tracing_subscriber::{fmt, EnvFilter};

use web_server_03::auth::JwtAuthService;
use web_server_03::handlers::{app, AppState};
use web_server_03::repository::RepositoryFactory;

#[derive(Clone, Debug)]
struct Config {
    port: u16,
    jwt_secret: String,
    jwt_exp_hours: i64,
    cors_allow_localhost: bool,
    max_page_size: u32,
    batch_limit: usize,
}

impl Config {
    fn from_env() -> Result<Self, String> {
        let port = std::env::var("PORT").ok().and_then(|s| s.parse::<u16>().ok()).unwrap_or(8080);
        let jwt_secret = std::env::var("JWT_SECRET").map_err(|_| "JWT_SECRET env var is required".to_string())?;
        let jwt_exp_hours = std::env::var("JWT_EXP_HOURS").ok().and_then(|s| s.parse::<i64>().ok()).unwrap_or(24);
        let cors_allow_localhost = true; // sensible default for dev
        let max_page_size = 100;
        let batch_limit = std::env::var("BATCH_LIMIT").ok().and_then(|s| s.parse::<usize>().ok()).unwrap_or(8);
        Ok(Self { port, jwt_secret, jwt_exp_hours, cors_allow_localhost, max_page_size, batch_limit })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Logging/Tracing setup. Pretty logs by default; override with RUST_LOG env.
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,axum=info,tower_http=info"));
    fmt().with_env_filter(env_filter).compact().init();

    let cfg = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    // Dependency injection: repository and auth service.
    let repo = RepositoryFactory::in_memory();
    let auth = Arc::new(JwtAuthService::new(&cfg.jwt_secret, cfg.jwt_exp_hours)) as Arc<dyn web_server_03::auth::AuthService>;

    let state = AppState { repo, auth, max_page_size: cfg.max_page_size, batch_limit: cfg.batch_limit };

    // Build the application router.
    let router: Router = app(state)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer(cfg.cors_allow_localhost));

    let addr = SocketAddr::from(([0, 0, 0, 0], cfg.port));
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Graceful shutdown: Ctrl+C or SIGTERM.
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn cors_layer(_allow_localhost: bool) -> CorsLayer {
    CorsLayer::new()
        .allow_origin([
            "http://localhost:3000".parse().unwrap(),
            "http://127.0.0.1:3000".parse().unwrap(),
        ])
        .allow_methods(Any)
        .allow_headers(Any)
}

async fn shutdown_signal() {
    // Wait for either Ctrl+C or a SIGTERM (Unix).
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate()).expect("failed to install signal handler");
        term.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! { _ = ctrl_c => {}, _ = terminate => {}, }
}
