use std::{collections::HashMap, net::SocketAddr, sync::{Arc, Mutex}};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Types
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: Uuid,
    name: String,
    email: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

// Shared state (immutably referenced across handlers)
type Database = Arc<Mutex<HashMap<Uuid, User>>>;

#[tokio::main]
async fn main() {
    // 🔒 Immutability: config and shared state bindings are immutable
    let database: Database = Arc::new(Mutex::new(HashMap::new()));
    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/users", post(create_user))
        .route("/users/:id", get(get_user))
        .with_state(database);

    // Immutable config for listener address
    let addr: SocketAddr = "127.0.0.1:3000".parse().expect("valid address");
    println!("🚀 Server running on http://{}", addr);
    println!("📋 Try these endpoints:\n   GET  /health\n   POST /users\n   GET  /users/<id>");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind TCP listener");
    axum::serve(listener, app).await.expect("server error");
}

// GET /health
async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
}

// GET /
async fn index() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({
        "message": "Rust Web Server 01",
        "endpoints": [
            "/health",
            "/users (POST)",
            "/users/<id> (GET)"
        ]
    })))
}

// POST /users
async fn create_user(
    State(database): State<Database>,
    Json(request): Json<CreateUserRequest>, // 🏠 Ownership: take ownership of request body
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // 👥 Borrowing: validate via &str
    let email = validate_email(&request.email)
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: e })))?;

    // 🔄 Shadowing: transform request fields into final types
    let id = Uuid::new_v4(); // generate ID

    // maybe trim name as a transform
    let name = request.name.trim().to_string();

    let user = User {
        id,
        name,
        email: email.to_string(),
    };

    // 👥 Borrowing DB for insertion
    let mut db = database
        .lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "Failed to lock database".into() })))?;

    // Insert moves user into the DB (ownership transfer inside HashMap)
    db.insert(user.id, user.clone());

    // 🏠 Ownership: move user to response
    Ok((StatusCode::CREATED, Json(user)))
}

// GET /users/:id
async fn get_user(
    State(database): State<Database>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // 🔄 Shadowing: String -> Uuid
    let id = Uuid::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Invalid UUID".into() })))?;

    // 👥 Borrowing DB for read (limit lock scope strictly)
    let user_opt = {
        let db = database
            .lock()
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "Failed to lock database".into() })))?;
        db.get(&id).cloned()
    };

    match user_opt {
        Some(user) => Ok((StatusCode::OK, Json(user))),
        None => Err((StatusCode::NOT_FOUND, Json(ErrorResponse { error: "User not found".into() })))
    }
}

// Validation using borrowing and Result
fn validate_email(email: &str) -> Result<&str, String> {
    // Very basic validation for demo
    if email.trim().is_empty() {
        return Err("Email cannot be empty".into());
    }
    if !email.contains('@') {
        return Err("Email must contain '@'".into());
    }
    Ok(email)
}
