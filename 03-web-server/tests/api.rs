use axum::{body::{self, Body}, http::{Request, StatusCode}, Router};
use serde_json::json;
use std::sync::Arc;
use tower::util::ServiceExt; // for `oneshot`

use web_server_03::handlers::{app, AppState};
use web_server_03::auth::{AuthService, JwtAuthService};
use web_server_03::repository::RepositoryFactory;

#[tokio::test]
async fn register_login_me_flow() {
    // Arrange: state with in-memory repo and deterministic JWT secret.
    let repo = RepositoryFactory::in_memory();
    let auth = Arc::new(JwtAuthService::new("testsecret", 24)) as Arc<dyn AuthService>;
    let state = AppState { repo, auth, max_page_size: 100, batch_limit: 4 };
    let app: Router = app(state.clone());

    // Register
    let payload = json!({ "email": "test@example.com", "password": "Password1" });
    let resp = app
        .clone()
        .oneshot(
            Request::post("/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Login
    let payload = json!({ "email": "test@example.com", "password": "Password1" });
    let mut resp = app
        .clone()
        .oneshot(
            Request::post("/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let token = v.get("token").and_then(|x| x.as_str()).unwrap().to_string();

    // Me
    let resp = app
        .oneshot(
            Request::get("/auth/me")
                .header("authorization", format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
