use std::sync::Arc;
use async_trait::async_trait;
use uuid::Uuid;
use sqlx::{PgPool, Row};
use crate::models::{AppError, User, UserStatus};

#[derive(Debug, Clone, Copy)]
pub struct ListOptions { pub page: u32, pub per_page: u32 }
impl ListOptions { pub fn clamp(self, max_per_page: u32) -> Self { let per = self.per_page.min(max_per_page).max(1); let page = self.page.max(1); Self { page, per_page: per } } }

#[derive(Debug, Clone, Default)]
pub struct UserStats { pub total: usize, pub active: usize, pub suspended: usize, pub pending: usize }

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: User) -> Result<User, AppError>;
    async fn find_by_id(&self, id: Uuid) -> Result<User, AppError>;
    async fn find_by_email(&self, email: &str) -> Result<User, AppError>;
    async fn list(&self, opts: ListOptions) -> Result<(Vec<User>, usize), AppError>;
    async fn update(&self, user: User) -> Result<User, AppError>;
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;
    async fn stats(&self) -> Result<UserStats, AppError>;
}

#[derive(Clone)]
pub struct PostgresUserRepository { pub pool: PgPool }
impl PostgresUserRepository { pub fn new(pool: PgPool) -> Self { Self { pool } } }

fn status_to_text(s: &UserStatus) -> String {
    match s {
        UserStatus::Active => "active".into(),
        UserStatus::Suspended { .. } => "suspended".into(),
        UserStatus::PendingVerification { .. } => "pending".into(),
    }
}
fn text_to_status(s: &str) -> UserStatus {
    match s {
        "active" => UserStatus::Active,
        "suspended" => UserStatus::Suspended { reason: "".into(), until: None },
        "pending" => UserStatus::PendingVerification { code: "".into() },
        _ => UserStatus::Active,
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn create(&self, user: User) -> Result<User, AppError> {
        let status = status_to_text(&user.status);
        let row = sqlx::query(
            r#"INSERT INTO users (id, email, password_hash, created_at, status)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id, email, password_hash, created_at, status"#,
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(user.created_at)
        .bind(status)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| if let sqlx::Error::Database(db) = &e { if db.is_unique_violation() { AppError::Conflict("email already exists".into()) } else { AppError::Repo(e.to_string()) } } else { AppError::Repo(e.to_string()) })?;
        Ok(User {
            id: row.get("id"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            created_at: row.get("created_at"),
            status: text_to_status(row.get::<String, _>("status").as_str()),
        })
    }

    async fn find_by_id(&self, id: Uuid) -> Result<User, AppError> {
        let row = sqlx::query(
            r#"SELECT id, email, password_hash, created_at, status FROM users WHERE id = $1"#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| AppError::NotFound("user not found".into()))?;
        Ok(User {
            id: row.get("id"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            created_at: row.get("created_at"),
            status: text_to_status(row.get::<String, _>("status").as_str()),
        })
    }

    async fn find_by_email(&self, email: &str) -> Result<User, AppError> {
        let row = sqlx::query(
            r#"SELECT id, email, password_hash, created_at, status FROM users WHERE lower(email) = lower($1)"#,
        )
        .bind(email)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| AppError::NotFound("user not found".into()))?;
        Ok(User {
            id: row.get("id"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            created_at: row.get("created_at"),
            status: text_to_status(row.get::<String, _>("status").as_str()),
        })
    }

    async fn list(&self, opts: ListOptions) -> Result<(Vec<User>, usize), AppError> {
        let offset = ((opts.page.saturating_sub(1)) as i64) * (opts.per_page as i64);
        let rows = sqlx::query(
            r#"SELECT id, email, password_hash, created_at, status
               FROM users ORDER BY created_at ASC
               LIMIT $1 OFFSET $2"#,
        )
        .bind(opts.per_page as i64)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Repo(e.to_string()))?;
        let count_row = sqlx::query("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Repo(e.to_string()))?;
        let total: i64 = count_row.get(0);
        let users = rows
            .into_iter()
            .map(|row| User {
                id: row.get("id"),
                email: row.get("email"),
                password_hash: row.get("password_hash"),
                created_at: row.get("created_at"),
                status: text_to_status(row.get::<String, _>("status").as_str()),
            })
            .collect();
        Ok((users, total as usize))
    }

    async fn update(&self, user: User) -> Result<User, AppError> {
        let status = status_to_text(&user.status);
        let row = sqlx::query(
            r#"UPDATE users SET email=$2, password_hash=$3, status=$4
               WHERE id=$1
               RETURNING id, email, password_hash, created_at, status"#,
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.password_hash)
        .bind(status)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Repo(e.to_string()))?;
        Ok(User {
            id: row.get("id"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            created_at: row.get("created_at"),
            status: text_to_status(row.get::<String, _>("status").as_str()),
        })
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        let rows = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Repo(e.to_string()))?;
        if rows.rows_affected() == 0 { return Err(AppError::NotFound("user not found".into())); }
        Ok(())
    }

    async fn stats(&self) -> Result<UserStats, AppError> {
        let rows = sqlx::query(
            r#"SELECT status, COUNT(*) as c FROM users GROUP BY status"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Repo(e.to_string()))?;
        let mut s = UserStats::default();
        let count_row = sqlx::query("SELECT COUNT(*) FROM users").fetch_one(&self.pool).await.map_err(|e| AppError::Repo(e.to_string()))?;
        s.total = count_row.get::<i64, _>(0) as usize;
        for row in rows {
            let status: String = row.get("status");
            let c: i64 = row.get("c");
            match status.as_str() {
                "active" => s.active = c as usize,
                "suspended" => s.suspended = c as usize,
                "pending" => s.pending = c as usize,
                _ => {}
            }
        }
        Ok(s)
    }
}

#[derive(Debug, Clone)]
pub struct RepositoryFactory;
impl RepositoryFactory {
    pub fn postgres(pool: PgPool) -> Arc<dyn UserRepository> { Arc::new(PostgresUserRepository::new(pool)) }
    pub fn in_memory() -> Arc<dyn UserRepository> { Arc::new(InMemoryUserRepository::new()) }
}

// In-memory repository for dev fallback
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug, Default)]
pub struct InMemoryUserRepository { inner: std::sync::Arc<RwLock<HashMap<Uuid, User>>> }
impl InMemoryUserRepository { pub fn new() -> Self { Self { inner: std::sync::Arc::new(RwLock::new(HashMap::new())) } } }

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn create(&self, user: User) -> Result<User, AppError> {
        let mut map = self.inner.write().await;
        if map.values().any(|u| u.email.eq_ignore_ascii_case(&user.email)) { return Err(AppError::Conflict("email already exists".into())); }
        map.insert(user.id, user.clone());
        Ok(user)
    }
    async fn find_by_id(&self, id: Uuid) -> Result<User, AppError> {
        let map = self.inner.read().await;
        map.get(&id).cloned().ok_or_else(|| AppError::NotFound("user not found".into()))
    }
    async fn find_by_email(&self, email: &str) -> Result<User, AppError> {
        let map = self.inner.read().await;
        map.values().find(|u| u.email.eq_ignore_ascii_case(email)).cloned().ok_or_else(|| AppError::NotFound("user not found".into()))
    }
    async fn list(&self, opts: ListOptions) -> Result<(Vec<User>, usize), AppError> {
        let map = self.inner.read().await;
        let mut users: Vec<User> = map.values().cloned().collect();
        users.sort_by_key(|u| u.created_at);
        let total = users.len();
        let start = ((opts.page.saturating_sub(1)) as usize).saturating_mul(opts.per_page as usize);
        let end = (start + opts.per_page as usize).min(total);
        let slice = if start < end { users[start..end].to_vec() } else { Vec::new() };
        Ok((slice, total))
    }
    async fn update(&self, user: User) -> Result<User, AppError> {
        let mut map = self.inner.write().await;
        if !map.contains_key(&user.id) { return Err(AppError::NotFound("user not found".into())); }
        map.insert(user.id, user.clone());
        Ok(user)
    }
    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        let mut map = self.inner.write().await;
        map.remove(&id).ok_or_else(|| AppError::NotFound("user not found".into()))?;
        Ok(())
    }
    async fn stats(&self) -> Result<UserStats, AppError> {
        let map = self.inner.read().await;
        let mut s = UserStats::default();
        s.total = map.len();
        for u in map.values() {
            match &u.status {
                UserStatus::Active => s.active += 1,
                UserStatus::Suspended { .. } => s.suspended += 1,
                UserStatus::PendingVerification { .. } => s.pending += 1,
            }
        }
        Ok(s)
    }
}
