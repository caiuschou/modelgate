use std::sync::Arc;

use super::error::ServiceError;

use super::repository::Repository;
use crate::db::ApiKeyAuthRow;

pub trait AuthService: Send + Sync {
    fn get_api_key_scope(&self, api_key: &str) -> Result<(i64, i64), ServiceError>;
    fn get_api_key_auth(&self, api_key: &str) -> Result<ApiKeyAuthRow, ServiceError>;
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

    fn get_api_key_auth(&self, api_key: &str) -> Result<ApiKeyAuthRow, ServiceError> {
        self.repo
            .get_api_key_auth(api_key)
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
        fn get_api_key_auth(
            &self,
            _api_key: &str,
        ) -> Result<crate::db::ApiKeyAuthRow, RepositoryError> {
            Err(RepositoryError::NotFound("api key not found".into()))
        }
        fn touch_api_key_last_used(
            &self,
            _key_id: i64,
            _now: i64,
        ) -> Result<(), RepositoryError> {
            Ok(())
        }
        fn ensure_monthly_quota(&self, _key_id: i64, _now: i64) -> Result<(), RepositoryError> {
            Ok(())
        }
        fn increment_quota_tokens(
            &self,
            _key_id: i64,
            _delta: i64,
        ) -> Result<(), RepositoryError> {
            Ok(())
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
        fn get_api_key_for_user(
            &self,
            _user_id: i64,
            _key_id: i64,
        ) -> Result<crate::services::repository::ApiKeySummary, RepositoryError> {
            Err(RepositoryError::NotFound("api key not found".into()))
        }
        fn insert_api_key_with_meta(
            &self,
            _user_id: i64,
            _api_key: &str,
            _created_at: u64,
            _name: &str,
            _description: &str,
            _expires_at: Option<i64>,
            _quota_monthly_tokens: Option<i64>,
            _model_allowlist: Option<&str>,
            _ip_allowlist: Option<&str>,
        ) -> Result<i64, RepositoryError> {
            Ok(1)
        }
        fn update_api_key_for_user(
            &self,
            _user_id: i64,
            _key_id: i64,
            _patch: &crate::db::ApiKeyPatchDb,
        ) -> Result<(), RepositoryError> {
            Ok(())
        }
        fn insert_api_key_audit(
            &self,
            _user_id: i64,
            _key_id: i64,
            _action: &str,
            _created_at: i64,
            _detail: Option<&str>,
        ) -> Result<(), RepositoryError> {
            Ok(())
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
