//! Authentication service providing password hashing (bcrypt) and JWT (HS256).
//! Demonstrates async traits and offloading CPU-bound work via spawn_blocking.

use std::{sync::Arc, time::{SystemTime, UNIX_EPOCH}};

use async_trait::async_trait;
use axum::http::HeaderMap;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use tokio::task;
use uuid::Uuid;

use crate::models::{AppError};

/// JWT claims payload; minimal for demo, add roles/permissions as needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user id as string UUID).
    pub sub: String,
    /// Issued at (seconds since epoch).
    pub iat: usize,
    /// Expiration (seconds since epoch).
    pub exp: usize,
}

#[async_trait]
pub trait AuthService: Send + Sync {
    async fn hash_password(&self, password: String) -> Result<String, AppError>;
    async fn verify_password(&self, password: String, hash: String) -> Result<bool, AppError>;
    async fn generate_token(&self, user_id: Uuid) -> Result<String, AppError>;
    async fn validate_token(&self, token: &str) -> Result<Claims, AppError>;
    async fn user_id_from_token(&self, token: &str) -> Result<Uuid, AppError> {
        let claims = self.validate_token(token).await?;
        Uuid::parse_str(&claims.sub).map_err(|e| AppError::Parse(e.to_string()))
    }
}

/// Concrete JWT/bcrypt implementation.
#[derive(Clone)]
pub struct JwtAuthService {
    encoding: EncodingKey,
    decoding: DecodingKey,
    expiry_hours: i64,
}

impl JwtAuthService {
    pub fn new(secret: &str, expiry_hours: i64) -> Self {
        Self { encoding: EncodingKey::from_secret(secret.as_bytes()), decoding: DecodingKey::from_secret(secret.as_bytes()), expiry_hours }
    }

    fn now_secs() -> usize { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as usize }
}

#[async_trait]
impl AuthService for JwtAuthService {
    async fn hash_password(&self, password: String) -> Result<String, AppError> {
        // Bcrypt is CPU-bound; use spawn_blocking to avoid blocking the async runtime.
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
        let claims = Claims { sub: user_id.to_string(), iat, exp };
        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)?;
        Ok(token)
    }

    async fn validate_token(&self, token: &str) -> Result<Claims, AppError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        let data = decode::<Claims>(token, &self.decoding, &validation)?;
        Ok(data.claims)
    }
}

/// Extract bearer token from Authorization header.
pub fn bearer_from_headers(headers: &HeaderMap) -> Option<String> {
    headers.get(axum::http::header::AUTHORIZATION).and_then(|h| h.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")).map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn token_round_trip() {
        let svc = JwtAuthService::new("secret", 1);
        let uid = Uuid::new_v4();
        let token = svc.generate_token(uid).await.unwrap();
        let claims = svc.validate_token(&token).await.unwrap();
        assert_eq!(claims.sub, uid.to_string());
    }

    #[tokio::test]
    async fn password_hash_verify() {
        let svc = JwtAuthService::new("secret", 1);
        let pwd = "Password1".to_string();
        let hash = svc.hash_password(pwd.clone()).await.unwrap();
        let ok = svc.verify_password(pwd, hash).await.unwrap();
        assert!(ok);
    }
}
