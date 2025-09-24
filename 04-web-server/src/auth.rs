use std::{sync::Arc, time::{SystemTime, UNIX_EPOCH}};
use async_trait::async_trait;
use axum::http::HeaderMap;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tokio::task;
use uuid::Uuid;

use crate::models::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iat: usize,
    pub exp: usize,
    pub jti: String,
}

#[async_trait]
pub trait AuthService: Send + Sync {
    async fn hash_password(&self, password: String) -> Result<String, AppError>;
    async fn verify_password(&self, password: String, hash: String) -> Result<bool, AppError>;
    async fn generate_token(&self, user_id: Uuid) -> Result<String, AppError>;
    async fn validate_token(&self, token: &str) -> Result<Claims, AppError>;
    async fn logout(&self, token: &str) -> Result<(), AppError>;
    async fn user_id_from_token(&self, token: &str) -> Result<Uuid, AppError> {
        let claims = self.validate_token(token).await?;
        Uuid::parse_str(&claims.sub).map_err(|e| AppError::Parse(e.to_string()))
    }
}

#[derive(Clone)]
pub struct HybridAuthService {
    encoding: EncodingKey,
    decoding: DecodingKey,
    expiry_hours: i64,
    redis: Option<redis::Client>,
}
impl HybridAuthService {
    pub fn new(secret: &str, expiry_hours: i64, redis: Option<redis::Client>) -> Self {
        Self { encoding: EncodingKey::from_secret(secret.as_bytes()), decoding: DecodingKey::from_secret(secret.as_bytes()), expiry_hours, redis }
    }
    fn now_secs() -> usize { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as usize }
}

#[async_trait]
impl AuthService for HybridAuthService {
    async fn hash_password(&self, password: String) -> Result<String, AppError> {
        let hashed = task::spawn_blocking(move || hash(password, DEFAULT_COST)).await.map_err(|e| AppError::Bcrypt(e.to_string()))??;
        Ok(hashed)
    }
    async fn verify_password(&self, password: String, hash_value: String) -> Result<bool, AppError> {
        let ok = task::spawn_blocking(move || verify(password, &hash_value)).await.map_err(|e| AppError::Bcrypt(e.to_string()))??;
        Ok(ok)
    }
    async fn generate_token(&self, user_id: Uuid) -> Result<String, AppError> {
        let iat = Self::now_secs();
        let exp = (Utc::now() + Duration::hours(self.expiry_hours)).timestamp() as usize;
        let jti = Uuid::new_v4().to_string();
        let claims = Claims { sub: user_id.to_string(), iat, exp, jti: jti.clone() };
        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)?;
        // whitelist jti in Redis with TTL
        let ttl_secs_i64 = (exp as i64 - iat as i64).max(0);
        let ttl: u64 = ttl_secs_i64.try_into().unwrap_or(0);
        if let Some(client) = &self.redis {
            let key = format!("jwt:{}", jti);
            let mut conn = client.get_async_connection().await.map_err(|e| AppError::Repo(e.to_string()))?;
            let _: () = conn.set_ex(key, "1", ttl).await.map_err(|e| AppError::Repo(e.to_string()))?;
        }
        Ok(token)
    }
    async fn validate_token(&self, token: &str) -> Result<Claims, AppError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        let data = decode::<Claims>(token, &self.decoding, &validation)?;
        if let Some(client) = &self.redis {
            let jti = data.claims.jti.clone();
            let key = format!("jwt:{}", jti);
            let mut conn = client.get_async_connection().await.map_err(|e| AppError::Repo(e.to_string()))?;
            let exists: bool = conn.exists(key).await.map_err(|e| AppError::Repo(e.to_string()))?;
            if !exists { return Err(AppError::Unauthorized("token revoked or expired".into())); }
        }
        Ok(data.claims)
    }
    async fn logout(&self, token: &str) -> Result<(), AppError> {
        if let Some(client) = &self.redis {
            let data = decode::<Claims>(token, &self.decoding, &Validation::new(Algorithm::HS256))?;
            let key = format!("jwt:{}", data.claims.jti);
            let mut conn = client.get_async_connection().await.map_err(|e| AppError::Repo(e.to_string()))?;
            let _: () = conn.del(key).await.map_err(|e| AppError::Repo(e.to_string()))?;
        }
        Ok(())
    }
}

pub fn bearer_from_headers(headers: &HeaderMap) -> Option<String> {
    headers.get(axum::http::header::AUTHORIZATION).and_then(|h| h.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")).map(|s| s.to_string())
}
