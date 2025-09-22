//! Repository layer with async trait abstraction and in-memory implementation.
//! Demonstrates zero-cost abstractions with traits and concurrency-safe shared state using Arc<RwLock<...>>.

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::models::{now, AppError, User, UserStatus};

/// Paginated listing options.
#[derive(Debug, Clone, Copy)]
pub struct ListOptions {
    pub page: u32,
    pub per_page: u32,
}

impl ListOptions {
    pub fn clamp(self, max_per_page: u32) -> Self {
        let per = self.per_page.min(max_per_page).max(1);
        let page = self.page.max(1);
        Self { page, per_page: per }
    }
}

/// User statistics summary.
#[derive(Debug, Clone, Default)]
pub struct UserStats {
    pub total: usize,
    pub active: usize,
    pub suspended: usize,
    pub pending: usize,
}

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

/// Simple in-memory repository for demonstration and tests.
#[derive(Debug, Default)]
pub struct InMemoryUserRepository {
    inner: Arc<RwLock<HashMap<Uuid, User>>>,
}

impl InMemoryUserRepository {
    pub fn new() -> Self { Self { inner: Arc::new(RwLock::new(HashMap::new())) } }
    pub fn shared(inner: Arc<RwLock<HashMap<Uuid, User>>>) -> Self { Self { inner } }
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn create(&self, user: User) -> Result<User, AppError> {
        let mut map = self.inner.write().await;
        if map.values().any(|u| u.email.eq_ignore_ascii_case(&user.email)) {
            return Err(AppError::Conflict("email already exists".into()));
        }
        map.insert(user.id, user.clone());
        Ok(user)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<User, AppError> {
        let map = self.inner.read().await;
        map.get(&id).cloned().ok_or_else(|| AppError::NotFound("user not found".into()))
    }

    async fn find_by_email(&self, email: &str) -> Result<User, AppError> {
        let map = self.inner.read().await;
        map.values()
            .find(|u| u.email.eq_ignore_ascii_case(email))
            .cloned()
            .ok_or_else(|| AppError::NotFound("user not found".into()))
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
        if !map.contains_key(&user.id) {
            return Err(AppError::NotFound("user not found".into()));
        }
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
        let mut stats = UserStats::default();
        stats.total = map.len();
        for u in map.values() {
            match &u.status {
                UserStatus::Active => stats.active += 1,
                UserStatus::Suspended { .. } => stats.suspended += 1,
                UserStatus::PendingVerification { .. } => stats.pending += 1,
            }
        }
        Ok(stats)
    }
}

/// Mock repository for unit tests; behavior can be customized as needed.
#[derive(Debug, Default, Clone)]
pub struct MockUserRepository;

#[async_trait]
impl UserRepository for MockUserRepository {
    async fn create(&self, user: User) -> Result<User, AppError> { Ok(user) }
    async fn find_by_id(&self, _id: Uuid) -> Result<User, AppError> {
        Err(AppError::NotFound("mock not implemented".into()))
    }
    async fn find_by_email(&self, _email: &str) -> Result<User, AppError> {
        Err(AppError::NotFound("mock not implemented".into()))
    }
    async fn list(&self, _opts: ListOptions) -> Result<(Vec<User>, usize), AppError> { Ok((vec![], 0)) }
    async fn update(&self, user: User) -> Result<User, AppError> { Ok(user) }
    async fn delete(&self, _id: Uuid) -> Result<(), AppError> { Ok(()) }
    async fn stats(&self) -> Result<UserStats, AppError> { Ok(UserStats::default()) }
}

/// Factory for repositories; in production could select between drivers.
#[derive(Debug, Clone)]
pub struct RepositoryFactory;

impl RepositoryFactory {
    pub fn in_memory() -> Arc<dyn UserRepository> { Arc::new(InMemoryUserRepository::new()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::UserStatus;

    #[tokio::test]
    async fn in_memory_crud_and_stats() {
        let repo = InMemoryUserRepository::new();
        let u1 = User { id: Uuid::new_v4(), email: "a@b.com".into(), password_hash: "hash".into(), created_at: now(), status: UserStatus::Active };
        let u2 = User { id: Uuid::new_v4(), email: "c@d.com".into(), password_hash: "hash".into(), created_at: now(), status: UserStatus::PendingVerification { code: "123".into() } };
        repo.create(u1.clone()).await.unwrap();
        repo.create(u2.clone()).await.unwrap();
        assert!(repo.find_by_email("a@b.com").await.is_ok());
        let (list, total) = repo.list(ListOptions { page: 1, per_page: 10 }).await.unwrap();
        assert_eq!(total, 2);
        assert_eq!(list.len(), 2);
        let stats = repo.stats().await.unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.active, 1);
        assert_eq!(stats.pending, 1);
    }
}
