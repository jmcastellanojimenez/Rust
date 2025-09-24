use axum::{http::StatusCode, response::{IntoResponse, Response}};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum UserStatus {
    Active,
    Suspended { reason: String, until: Option<DateTime<Utc>> },
    PendingVerification { code: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub status: UserStatus,
}

impl User {
    pub fn validate_email(email: &str) -> Result<(), AppError> {
        let has_at = email.contains('@');
        let has_dot = email.contains('.');
        let ok_len = email.len() <= 254 && email.len() >= 3;
        if has_at && has_dot && ok_len { Ok(()) } else { Err(AppError::Validation("invalid email format".into())) }
    }
    pub fn validate_password_policy(password: &str) -> Result<(), AppError> {
        if password.len() < 8 { return Err(AppError::Validation("password too short (min 8)".into())); }
        let has_letter = password.chars().any(|c| c.is_ascii_alphabetic());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        if has_letter && has_digit { Ok(()) } else { Err(AppError::Validation("password must include at least one letter and one number".into())) }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ApiResponse<T>
where T: Serialize {
    Success(T),
    Error { message: String },
}
impl<T> ApiResponse<T> where T: Serialize {
    pub fn success(data: T) -> Self { Self::Success(data) }
    pub fn error<M: Into<String>>(message: M) -> Self { Self::Error { message: message.into() } }
}
impl<T> IntoResponse for ApiResponse<T> where T: Serialize {
    fn into_response(self) -> Response {
        match self {
            ApiResponse::Success(payload) => (StatusCode::OK, axum::Json(payload)).into_response(),
            ApiResponse::Error { message } => (StatusCode::BAD_REQUEST, axum::Json(serde_json::json!({"error": message}))).into_response(),
        }
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("validation error: {0}")] Validation(String),
    #[error("not found: {0}")] NotFound(String),
    #[error("conflict: {0}")] Conflict(String),
    #[error("unauthorized: {0}")] Unauthorized(String),
    #[error("forbidden: {0}")] Forbidden(String),
    #[error("jwt error: {0}")] Jwt(String),
    #[error("password error: {0}")] Bcrypt(String),
    #[error("repository error: {0}")] Repo(String),
    #[error("parse error: {0}")] Parse(String),
    #[error("unknown error: {0}")] Unknown(String),
}
impl AppError { pub fn status_code(&self) -> StatusCode { match self {
    AppError::Validation(_) => StatusCode::BAD_REQUEST,
    AppError::NotFound(_) => StatusCode::NOT_FOUND,
    AppError::Conflict(_) => StatusCode::CONFLICT,
    AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
    AppError::Forbidden(_) => StatusCode::FORBIDDEN,
    AppError::Jwt(_) | AppError::Bcrypt(_) | AppError::Repo(_) | AppError::Parse(_) => StatusCode::INTERNAL_SERVER_ERROR,
    AppError::Unknown(_) => StatusCode::INTERNAL_SERVER_ERROR,
}}}
impl IntoResponse for AppError { fn into_response(self) -> Response { let status = self.status_code(); let body = serde_json::json!({"error": self.to_string()}); (status, axum::Json(body)).into_response() } }
impl From<bcrypt::BcryptError> for AppError { fn from(e: bcrypt::BcryptError) -> Self { AppError::Bcrypt(e.to_string()) } }
impl From<jsonwebtoken::errors::Error> for AppError { fn from(e: jsonwebtoken::errors::Error) -> Self { AppError::Jwt(e.to_string()) } }
impl From<anyhow::Error> for AppError { fn from(e: anyhow::Error) -> Self { AppError::Unknown(e.to_string()) } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest { pub email: String, pub password: String }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest { pub email: String, pub password: String }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse { pub id: Uuid, pub email: String, pub created_at: DateTime<Utc>, pub status: UserStatus }
impl From<User> for UserResponse { fn from(u: User) -> Self { Self { id: u.id, email: u.email, created_at: u.created_at, status: u.status } } }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paginated<T> { pub items: Vec<T>, pub page: u32, pub per_page: u32, pub total: usize }

pub fn generate_demo_verification_code() -> String { "123456".to_string() }
pub fn now() -> DateTime<Utc> { Utc::now() }
pub fn hours_from_now(h: i64) -> DateTime<Utc> { Utc::now() + Duration::hours(h) }
