//! Domain models, DTOs, and error/response types for the 03-web-server application.
//! This module demonstrates idiomatic Rust with:
//! - Strongly typed domain models
//! - Serde for JSON (de)serialization
//! - thiserror for ergonomic error types
//! - Implementations of Axum IntoResponse for type-safe HTTP responses

use std::fmt;

use axum::{http::StatusCode, response::{IntoResponse, Response}};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Represents the state of a user account.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum UserStatus {
    /// Fully active account.
    Active,
    /// Temporarily suspended with a reason and an optional until time.
    Suspended {
        reason: String,
        /// When the suspension is lifted. None means indefinite until admin action.
        until: Option<DateTime<Utc>>,
    },
    /// Pending email/phone verification with a code.
    PendingVerification {
        code: String,
    },
}

/// Core user domain model as it would be persisted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    /// Hash of the user's password (never store plaintext).
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub status: UserStatus,
}

impl User {
    /// Validate user fields for invariants that must always hold.
    pub fn validate_email(email: &str) -> Result<(), AppError> {
        // Extremely simple email check for demo purposes.
        let has_at = email.contains('@');
        let has_dot = email.contains('.');
        let ok_len = email.len() <= 254 && email.len() >= 3;
        if has_at && has_dot && ok_len {
            Ok(())
        } else {
            Err(AppError::Validation("invalid email format".into()))
        }
    }

    /// Enforces password policy: >=8 chars and contains at least one letter and one digit.
    pub fn validate_password_policy(password: &str) -> Result<(), AppError> {
        if password.len() < 8 {
            return Err(AppError::Validation("password too short (min 8)".into()));
        }
        let has_letter = password.chars().any(|c| c.is_ascii_alphabetic());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        if has_letter && has_digit {
            Ok(())
        } else {
            Err(AppError::Validation(
                "password must include at least one letter and one number".into(),
            ))
        }
    }
}

/// Generic API response wrapper used to standardize success and error shapes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ApiResponse<T>
where
    T: Serialize,
{
    Success(T),
    Error { message: String },
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn success(data: T) -> Self { Self::Success(data) }
    pub fn error<M: Into<String>>(message: M) -> Self { Self::Error { message: message.into() } }
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        match self {
            ApiResponse::Success(payload) => (StatusCode::OK, axum::Json(payload)).into_response(),
            ApiResponse::Error { message } => {
                (StatusCode::BAD_REQUEST, axum::Json(serde_json::json!({ "error": message }))).into_response()
            }
        }
    }
}

/// Application-wide error type with variants mapped to HTTP status codes.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("jwt error: {0}")]
    Jwt(String),
    #[error("password error: {0}")]
    Bcrypt(String),
    #[error("repository error: {0}")]
    Repo(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("unknown error: {0}")]
    Unknown(String),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::Jwt(_) | AppError::Bcrypt(_) | AppError::Repo(_) | AppError::Parse(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            AppError::Unknown(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = serde_json::json!({
            "error": self.to_string(),
        });
        (status, axum::Json(body)).into_response()
    }
}

impl From<bcrypt::BcryptError> for AppError {
    fn from(e: bcrypt::BcryptError) -> Self { AppError::Bcrypt(e.to_string()) }
}
impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(e: jsonwebtoken::errors::Error) -> Self { AppError::Jwt(e.to_string()) }
}
impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self { AppError::Unknown(e.to_string()) }
}

/// Requests and Responses

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub status: UserStatus,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self { id: u.id, email: u.email, created_at: u.created_at, status: u.status }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total: usize,
}

/// Utility to compute a default verification code for demo.
pub fn generate_demo_verification_code() -> String { "123456".to_string() }

/// Simple helpers for generating timestamps.
pub fn now() -> DateTime<Utc> { Utc::now() }

/// Helper to produce expiry times.
pub fn hours_from_now(h: i64) -> DateTime<Utc> { Utc::now() + Duration::hours(h) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_validation_works() {
        assert!(User::validate_email("a@b.com").is_ok());
        assert!(User::validate_email("invalid").is_err());
    }

    #[test]
    fn password_policy_works() {
        assert!(User::validate_password_policy("Password1").is_ok());
        assert!(User::validate_password_policy("short").is_err());
        assert!(User::validate_password_policy("allletters").is_err());
        assert!(User::validate_password_policy("12345678").is_err());
    }
}
