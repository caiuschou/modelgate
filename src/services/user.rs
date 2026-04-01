use std::sync::Arc;

use super::error::ServiceError;
use super::repository::Repository;

pub trait UserService: Send + Sync {
    fn create_user_with_api_key(
        &self,
        username: &str,
        api_key: &str,
        created_at: u64,
    ) -> Result<(), ServiceError>;
    fn create_api_key_for_user(
        &self,
        username: &str,
        api_key: &str,
        created_at: u64,
    ) -> Result<(), ServiceError>;
    fn register_user_with_password_and_api_key(
        &self,
        username: &str,
        password_hash: &str,
        api_key: &str,
        created_at: u64,
    ) -> Result<(), ServiceError>;
    fn get_user_login_credentials(
        &self,
        username: &str,
    ) -> Result<Option<(i64, Option<String>)>, ServiceError>;
    fn get_first_api_key_for_user(&self, user_id: i64) -> Result<Option<String>, ServiceError>;
    fn create_api_key_for_user_id(
        &self,
        user_id: i64,
        api_key: &str,
        created_at: u64,
    ) -> Result<(), ServiceError>;
}

pub struct DefaultUserService {
    repo: Arc<dyn Repository>,
}

impl DefaultUserService {
    pub fn new(repo: Arc<dyn Repository>) -> Self {
        Self { repo }
    }
}

impl UserService for DefaultUserService {
    fn create_user_with_api_key(
        &self,
        username: &str,
        api_key: &str,
        created_at: u64,
    ) -> Result<(), ServiceError> {
        self.repo
            .create_user_with_api_key(username, api_key, created_at)
            .map_err(ServiceError::from)
    }

    fn create_api_key_for_user(
        &self,
        username: &str,
        api_key: &str,
        created_at: u64,
    ) -> Result<(), ServiceError> {
        self.repo
            .create_api_key_for_user(username, api_key, created_at)
            .map_err(ServiceError::from)
    }

    fn register_user_with_password_and_api_key(
        &self,
        username: &str,
        password_hash: &str,
        api_key: &str,
        created_at: u64,
    ) -> Result<(), ServiceError> {
        self.repo
            .register_user_with_password_and_api_key(username, password_hash, api_key, created_at)
            .map_err(ServiceError::from)
    }

    fn get_user_login_credentials(
        &self,
        username: &str,
    ) -> Result<Option<(i64, Option<String>)>, ServiceError> {
        self.repo
            .get_user_login_credentials(username)
            .map_err(ServiceError::from)
    }

    fn get_first_api_key_for_user(&self, user_id: i64) -> Result<Option<String>, ServiceError> {
        self.repo
            .get_first_api_key_for_user(user_id)
            .map_err(ServiceError::from)
    }

    fn create_api_key_for_user_id(
        &self,
        user_id: i64,
        api_key: &str,
        created_at: u64,
    ) -> Result<(), ServiceError> {
        self.repo
            .create_api_key_for_user_id(user_id, api_key, created_at)
            .map_err(ServiceError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{AuditListItem, AuditListQuery, AuditRecord};
    use crate::services::error::RepositoryError;

    struct ConflictRepo;

    impl Repository for ConflictRepo {
        fn get_api_key_info(&self, _api_key: &str) -> Result<(i64, i64), RepositoryError> {
            Err(RepositoryError::NotFound("api key not found".into()))
        }
        fn query_audit_logs(
            &self,
            _query: &AuditListQuery,
            _scoped_user_id: Option<i64>,
        ) -> Result<(Vec<AuditListItem>, i64), RepositoryError> {
            Ok((Vec::new(), 0))
        }
        fn get_audit_log_by_request_id(
            &self,
            _request_id: &str,
            _scoped_user_id: Option<i64>,
        ) -> Result<AuditRecord, RepositoryError> {
            Err(RepositoryError::NotFound("audit log not found".into()))
        }
        fn create_user_with_api_key(
            &self,
            _username: &str,
            _api_key: &str,
            _created_at: u64,
        ) -> Result<(), RepositoryError> {
            Err(RepositoryError::Conflict("username already exists".into()))
        }
        fn create_api_key_for_user(
            &self,
            _username: &str,
            _api_key: &str,
            _created_at: u64,
        ) -> Result<(), RepositoryError> {
            Ok(())
        }
        fn register_user_with_password_and_api_key(
            &self,
            _username: &str,
            _password_hash: &str,
            _api_key: &str,
            _created_at: u64,
        ) -> Result<(), RepositoryError> {
            Ok(())
        }
        fn get_user_login_credentials(
            &self,
            _username: &str,
        ) -> Result<Option<(i64, Option<String>)>, RepositoryError> {
            Ok(None)
        }
        fn get_first_api_key_for_user(
            &self,
            _user_id: i64,
        ) -> Result<Option<String>, RepositoryError> {
            Ok(None)
        }
        fn create_api_key_for_user_id(
            &self,
            _user_id: i64,
            _api_key: &str,
            _created_at: u64,
        ) -> Result<(), RepositoryError> {
            Ok(())
        }
    }

    #[test]
    fn user_service_propagates_conflict_message() {
        let service = DefaultUserService::new(Arc::new(ConflictRepo));
        let err = service
            .create_user_with_api_key("alice", "sk-1", 1)
            .expect_err("should fail with conflict");
        match err {
            ServiceError::Conflict(msg) => assert!(msg.contains("already exists")),
            _ => panic!("unexpected error variant"),
        }
    }
}

