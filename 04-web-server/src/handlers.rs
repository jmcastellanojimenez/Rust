use std::sync::Arc;
use axum::{debug_handler, extract::{Query, State}, http::StatusCode, response::IntoResponse, routing::{get, post}, Json, Router};
use futures::future::join_all;
use serde::Deserialize;
use tokio::sync::Semaphore;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::{auth::{bearer_from_headers, AuthService}, models::{AppError, Paginated, RegisterRequest, LoginRequest, User, UserResponse, UserStatus, ApiResponse, now, generate_demo_verification_code}, repository::{ListOptions, UserRepository}};

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<dyn UserRepository>,
    pub auth: Arc<dyn AuthService>,
    pub max_page_size: u32,
    pub batch_limit: usize,
    pub db: Option<sqlx::PgPool>,
    pub redis: Option<redis::Client>,
}

pub fn app(state: AppState) -> Router {
    let auth_routes = Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/me", get(me));

    let user_routes = Router::new()
        .route("/", get(list_users))
        .route("/stats", get(user_stats))
        .route("/batch", post(batch_create_users));

    Router::new()
        .nest("/auth", auth_routes)
        .nest("/users", user_routes)
        .route("/healthz", get(health))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

#[derive(Debug, Deserialize)]
struct PaginationQuery { page: Option<u32>, per_page: Option<u32> }

#[debug_handler]
pub async fn register(State(state): State<AppState>, Json(payload): Json<RegisterRequest>) -> Result<impl IntoResponse, AppError> {
    crate::models::User::validate_email(&payload.email)?;
    crate::models::User::validate_password_policy(&payload.password)?;
    let email = payload.email.to_lowercase();
    let password_hash = state.auth.hash_password(payload.password).await?;
    let user = User { id: Uuid::new_v4(), email, password_hash, created_at: now(), status: UserStatus::PendingVerification { code: generate_demo_verification_code() } };
    let user = state.repo.create(user).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(UserResponse::from(user)))))
}

#[debug_handler]
pub async fn login(State(state): State<AppState>, Json(payload): Json<LoginRequest>) -> Result<impl IntoResponse, AppError> {
    let user = state.repo.find_by_email(&payload.email).await.map_err(|_| AppError::Unauthorized("invalid credentials".into()))?;
    let ok = state.auth.verify_password(payload.password, user.password_hash.clone()).await?;
    if !ok { return Err(AppError::Unauthorized("invalid credentials".into())); }
    let token = state.auth.generate_token(user.id).await?;
    Ok(Json(serde_json::json!({ "token": token })).into_response())
}

async fn current_user_from_headers(state: &AppState, headers: &axum::http::HeaderMap) -> Result<User, AppError> {
    let token = bearer_from_headers(headers).ok_or_else(|| AppError::Unauthorized("missing bearer token".into()))?;
    let user_id = state.auth.user_id_from_token(&token).await?;
    let user = state.repo.find_by_id(user_id).await?;
    Ok(user)
}

pub async fn me(State(state): State<AppState>, headers: axum::http::HeaderMap) -> Result<impl IntoResponse, AppError> {
    let user = current_user_from_headers(&state, &headers).await?;
    Ok(Json(UserResponse::from(user)))
}

pub async fn list_users(State(state): State<AppState>, Query(pq): Query<PaginationQuery>) -> Result<impl IntoResponse, AppError> {
    let page = pq.page.unwrap_or(1);
    let per_page = pq.per_page.unwrap_or(20);
    let opts = ListOptions { page, per_page }.clamp(state.max_page_size);
    let (users, total) = state.repo.list(opts).await?;
    let items: Vec<UserResponse> = users.into_iter().map(UserResponse::from).collect();
    Ok(Json(Paginated { items, page: opts.page, per_page: opts.per_page, total }))
}

pub async fn user_stats(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let stats = state.repo.stats().await?;
    Ok(Json(serde_json::json!({ "total": stats.total, "active": stats.active, "suspended": stats.suspended, "pending": stats.pending })))
}

pub async fn batch_create_users(State(state): State<AppState>, Json(items): Json<Vec<RegisterRequest>>) -> Result<impl IntoResponse, AppError> {
    let semaphore = Arc::new(Semaphore::new(state.batch_limit));
    let futures = items.into_iter().map(|req| {
        let state = state.clone();
        let semaphore = semaphore.clone();
        async move {
            let _permit = semaphore.acquire().await.map_err(|e| AppError::Unknown(e.to_string()))?;
            crate::models::User::validate_email(&req.email)?;
            crate::models::User::validate_password_policy(&req.password)?;
            let email = req.email.to_lowercase();
            let password_hash = state.auth.hash_password(req.password).await?;
            let user = User { id: Uuid::new_v4(), email, password_hash, created_at: now(), status: UserStatus::Active };
            state.repo.create(user).await
        }
    });
    let results = join_all(futures).await;
    let mut created: Vec<UserResponse> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    for r in results { match r { Ok(u) => created.push(UserResponse::from(u)), Err(e) => errors.push(e.to_string()) } }
    Ok(Json(serde_json::json!({ "created": created, "errors": errors })))
}

pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    // Check Postgres if available
    let pg_ok = if let Some(ref pool) = state.db {
        sqlx::query("SELECT 1").fetch_one(pool).await.is_ok()
    } else { false };

    // Check Redis if available
    let redis_ok = if let Some(ref client) = state.redis {
        match client.get_async_connection().await {
            Ok(mut conn) => redis::cmd("PING").query_async::<_, String>(&mut conn).await.is_ok(),
            Err(_) => false,
        }
    } else { false };

    if pg_ok && redis_ok {
        (StatusCode::OK, Json(serde_json::json!({"status":"ok"})))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"status":"degraded","postgres": pg_ok, "redis": redis_ok })))
    }
}
