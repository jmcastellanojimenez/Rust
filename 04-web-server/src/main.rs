use std::{net::SocketAddr, sync::Arc};
use axum::Router;
use tower_http::{cors::{Any, CorsLayer}, compression::CompressionLayer, trace::TraceLayer};
use tracing_subscriber::EnvFilter;
use sqlx::postgres::PgPoolOptions;

use web_server_04::auth::{AuthService, HybridAuthService};
use web_server_04::handlers::{app, AppState};
use web_server_04::repository::RepositoryFactory;
use web_server_04::config::AppConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,axum=info,tower_http=info"));
    tracing_subscriber::fmt().with_env_filter(env_filter).compact().init();

    let cfg = match AppConfig::from_env() { Ok(c) => c, Err(e) => { eprintln!("Configuration error: {}", e); std::process::exit(1);} };

    // Try to connect to Postgres; fall back to in-memory if unavailable
    let pool = match PgPoolOptions::new()
        .max_connections(cfg.database.max_connections)
        .connect(&cfg.database.url)
        .await
    {
        Ok(p) => {
            // Run migrations
            if let Err(e) = sqlx::migrate!("./migrations").run(&p).await {
                tracing::error!(error = %e, "migrations failed; continuing without database");
                // If migrations fail, treat as no DB
                None
            } else {
                Some(p)
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "Postgres not available; starting with in-memory repository");
            None
        }
    };

    // Try Redis; continue without it if unavailable
    let redis_client = match redis::Client::open(cfg.redis.url.clone()) {
        Ok(client) => {
            match client.get_async_connection().await {
                Ok(mut conn) => {
                    let ping_ok = redis::cmd("PING").query_async::<_, String>(&mut conn).await.is_ok();
                    if ping_ok { Some(client) } else {
                        tracing::warn!("Redis PING failed; continuing without Redis");
                        None
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Redis connection failed; continuing without Redis");
                    None
                }
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "Redis client init failed; continuing without Redis");
            None
        }
    };

    // DI wiring: choose repo based on DB availability
    let repo: std::sync::Arc<dyn web_server_04::repository::UserRepository> = if let Some(ref p) = pool {
        RepositoryFactory::postgres(p.clone())
    } else {
        RepositoryFactory::in_memory()
    };

    let auth = Arc::new(HybridAuthService::new(&cfg.jwt.secret, cfg.jwt.expiry_hours, redis_client.clone())) as Arc<dyn AuthService>;

    let state = AppState { repo, auth, max_page_size: cfg.max_page_size, batch_limit: cfg.batch_limit, db: pool.clone(), redis: redis_client.clone() };

    let router: Router = app(state)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer());

    let addr = SocketAddr::from(([0, 0, 0, 0], cfg.server.port));
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).with_graceful_shutdown(shutdown_signal()).await?;
    Ok(())
}

fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin([
            "http://localhost:3000".parse().unwrap(),
            "http://127.0.0.1:3000".parse().unwrap(),
        ])
        .allow_methods(Any)
        .allow_headers(Any)
}

async fn shutdown_signal() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler"); };
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
