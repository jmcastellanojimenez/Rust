use crate::models::AppError;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Clone, Debug)]
pub struct JwtConfig {
    pub secret: String,
    pub expiry_hours: i64,
    pub algorithm: String, // HS256 for now
}

#[derive(Clone, Debug)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Clone, Debug)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub jwt: JwtConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub max_page_size: u32,
    pub batch_limit: usize,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        use std::env;
        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("PORT").ok().and_then(|s| s.parse::<u16>().ok()).unwrap_or(8080);
        let jwt_secret = env::var("JWT_SECRET").map_err(|_| AppError::Validation("JWT_SECRET is required".into()))?;
        if jwt_secret.len() < 32 { return Err(AppError::Validation("JWT_SECRET must be at least 32 characters".into())); }
        let jwt_expiry_hours = env::var("JWT_EXPIRY_HOURS").or_else(|_| env::var("JWT_EXP_HOURS")).ok().and_then(|s| s.parse::<i64>().ok()).unwrap_or(24);
        let algorithm = "HS256".to_string();
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://user:password@localhost:5432/app".to_string());
        let db_max = env::var("DB_MAX_CONNECTIONS").ok().and_then(|s| s.parse::<u32>().ok()).unwrap_or(20);
        let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let max_page_size = env::var("MAX_PAGE_SIZE").ok().and_then(|s| s.parse::<u32>().ok()).unwrap_or(100);
        let batch_limit = env::var("BATCH_LIMIT").ok().and_then(|s| s.parse::<usize>().ok()).unwrap_or(8);
        Ok(Self {
            server: ServerConfig { host, port },
            jwt: JwtConfig { secret: jwt_secret, expiry_hours: jwt_expiry_hours, algorithm },
            database: DatabaseConfig { url: database_url, max_connections: db_max },
            redis: RedisConfig { url: redis_url },
            max_page_size,
            batch_limit,
        })
    }
}
