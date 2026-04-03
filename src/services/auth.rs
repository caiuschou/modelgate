use std::sync::Arc;

use super::error::ServiceError;

use super::repository::Repository;

pub trait AuthService: Send + Sync {
    fn get_api_key_scope(&self, api_key: &str) -> Result<(i64, i64), ServiceError>;
}

pub struct DefaultAuthService {
    repo: Arc<dyn Repository>,
}

impl DefaultAuthService {
    pub fn new(repo: Arc<dyn Repository>) -> Self {
        Self { repo }
    }
}

impl AuthService for DefaultAuthService {
    fn get_api_key_scope(&self, api_key: &str) -> Result<(i64, i64), ServiceError> {
        self.repo
            .get_api_key_info(api_key)
            .map_err(|_| ServiceError::Unauthorized("Invalid or missing API key".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{AuditListItem, AuditListQuery, AuditRecord};
    use crate::services::error::RepositoryError;

    struct MockRepo;

    impl Repository for MockRepo {
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
            Ok(())
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
        fn list_api_keys_for_user(
            &self,
            _user_id: i64,
        ) -> Result<Vec<crate::services::repository::ApiKeySummary>, RepositoryError> {
            Ok(Vec::new())
        }
        fn insert_api_key_for_user_returning_id(
            &self,
            _user_id: i64,
            _api_key: &str,
            _created_at: u64,
        ) -> Result<i64, RepositoryError> {
            Ok(1)
        }
        fn revoke_api_key_for_user(
            &self,
            _user_id: i64,
            _key_id: i64,
        ) -> Result<(), RepositoryError> {
            Ok(())
        }
    }

    #[test]
    fn auth_service_maps_missing_key_to_unauthorized() {
        let service = DefaultAuthService::new(Arc::new(MockRepo));
        let err = service
            .get_api_key_scope("missing")
            .expect_err("should fail for missing key");
        match err {
            ServiceError::Unauthorized(msg) => assert!(msg.contains("Invalid or missing API key")),
            _ => panic!("unexpected error variant"),
        }
    }
}
