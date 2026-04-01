pub mod audit;
pub mod auth;
pub mod error;
pub mod repository;
pub mod user;

use std::sync::Arc;

pub use audit::{AuditService, DefaultAuditService};
pub use auth::{AuthService, DefaultAuthService};
pub use error::ServiceError;
pub use user::{DefaultUserService, UserService};

#[derive(Clone)]
pub struct ServiceContainer {
    pub auth: Arc<dyn AuthService>,
    pub audit: Arc<dyn AuditService>,
    pub user: Arc<dyn UserService>,
}

pub fn build_service_container(db: crate::db::DbConn) -> ServiceContainer {
    let repo: Arc<dyn repository::Repository> = Arc::new(repository::SqliteRepository::new(db));
    ServiceContainer {
        auth: Arc::new(DefaultAuthService::new(repo.clone())),
        audit: Arc::new(DefaultAuditService::new(repo.clone())),
        user: Arc::new(DefaultUserService::new(repo)),
    }
}

