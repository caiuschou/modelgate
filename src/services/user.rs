use std::sync::Arc;

use super::error::RepositoryError;
use super::error::ServiceError;
use super::repository::{ApiKeySummary, Repository};
use crate::db::ApiKeyPatchDb;

/// Console create-key payload (validated in service).
#[derive(Debug, Clone, Default)]
pub struct CreateMyApiKeyInput {
    pub name: String,
    pub description: Option<String>,
    pub expires_at: Option<u64>,
    pub quota_monthly_tokens: Option<i64>,
    pub model_allowlist: Option<Vec<String>>,
    pub ip_allowlist: Option<Vec<String>>,
}

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

    fn list_my_api_keys(&self, user_id: i64) -> Result<Vec<ApiKeySummary>, ServiceError>;

    /// Returns `(id, full_api_key, created_at)` — full key only at creation time.
    fn create_my_api_key(
        &self,
        user_id: i64,
        created_at: u64,
        input: CreateMyApiKeyInput,
    ) -> Result<(i64, String, u64), ServiceError>;

    fn get_my_api_key(&self, user_id: i64, key_id: i64) -> Result<ApiKeySummary, ServiceError>;

    fn update_my_api_key(
        &self,
        user_id: i64,
        key_id: i64,
        patch: ApiKeyPatchDb,
    ) -> Result<(), ServiceError>;

    fn revoke_my_api_key(&self, user_id: i64, key_id: i64) -> Result<(), ServiceError>;

    fn touch_api_key_last_used(&self, key_id: i64, now: i64) -> Result<(), ServiceError>;
    fn ensure_monthly_quota(&self, key_id: i64, now: i64) -> Result<(), ServiceError>;
    fn increment_quota_tokens(&self, key_id: i64, delta: i64) -> Result<(), ServiceError>;
}

pub struct DefaultUserService {
    repo: Arc<dyn Repository>,
}

impl DefaultUserService {
    pub fn new(repo: Arc<dyn Repository>) -> Self {
        Self { repo }
    }
}

fn validate_create_input(input: &CreateMyApiKeyInput) -> Result<(), ServiceError> {
    let n = input.name.trim();
    if n.is_empty() || n.len() > 64 {
        return Err(ServiceError::BadRequest(
            "name is required and must be at most 64 characters".into(),
        ));
    }
    if let Some(ref d) = input.description {
        if d.len() > 512 {
            return Err(ServiceError::BadRequest(
                "description must be at most 512 characters".into(),
            ));
        }
    }
    if let Some(q) = input.quota_monthly_tokens {
        if q <= 0 {
            return Err(ServiceError::BadRequest(
                "quota_monthly_tokens must be positive when set".into(),
            ));
        }
    }
    Ok(())
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

    fn list_my_api_keys(&self, user_id: i64) -> Result<Vec<ApiKeySummary>, ServiceError> {
        self.repo
            .list_api_keys_for_user(user_id)
            .map_err(ServiceError::from)
    }

    fn create_my_api_key(
        &self,
        user_id: i64,
        created_at: u64,
        input: CreateMyApiKeyInput,
    ) -> Result<(i64, String, u64), ServiceError> {
        validate_create_input(&input)?;
        let api_key = generate_api_key_string();
        let name = input.name.trim().to_string();
        let description = input.description.unwrap_or_default();
        let expires_at = input.expires_at.map(|u| u as i64);
        let model_json = input
            .model_allowlist
            .as_ref()
            .filter(|v| !v.is_empty())
            .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".into()));
        let ip_json = input
            .ip_allowlist
            .as_ref()
            .filter(|v| !v.is_empty())
            .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".into()));
        let id = self.repo.insert_api_key_with_meta(
            user_id,
            &api_key,
            created_at,
            &name,
            &description,
            expires_at,
            input.quota_monthly_tokens,
            model_json.as_deref(),
            ip_json.as_deref(),
        )?;
        Ok((id, api_key, created_at))
    }

    fn get_my_api_key(&self, user_id: i64, key_id: i64) -> Result<ApiKeySummary, ServiceError> {
        self.repo
            .get_api_key_for_user(user_id, key_id)
            .map_err(ServiceError::from)
    }

    fn update_my_api_key(
        &self,
        user_id: i64,
        key_id: i64,
        patch: ApiKeyPatchDb,
    ) -> Result<(), ServiceError> {
        self.repo
            .update_api_key_for_user(user_id, key_id, &patch)
            .map_err(ServiceError::from)
    }

    fn revoke_my_api_key(&self, user_id: i64, key_id: i64) -> Result<(), ServiceError> {
        self.repo
            .revoke_api_key_for_user(user_id, key_id)
            .map_err(ServiceError::from)
    }

    fn touch_api_key_last_used(&self, key_id: i64, now: i64) -> Result<(), ServiceError> {
        self.repo
            .touch_api_key_last_used(key_id, now)
            .map_err(ServiceError::from)
    }

    fn ensure_monthly_quota(&self, key_id: i64, now: i64) -> Result<(), ServiceError> {
        match self.repo.ensure_monthly_quota(key_id, now) {
            Ok(()) => Ok(()),
            Err(RepositoryError::Forbidden(m)) if m.contains("quota") => {
                Err(ServiceError::TooManyRequests(m))
            }
            Err(e) => Err(e.into()),
        }
    }

    fn increment_quota_tokens(&self, key_id: i64, delta: i64) -> Result<(), ServiceError> {
        self.repo
            .increment_quota_tokens(key_id, delta)
            .map_err(ServiceError::from)
    }
}

fn generate_api_key_string() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random_part: String = (0..32)
        .map(|_| format!("{:x}", rng.gen::<u8>() % 16))
        .collect();
    format!("sk-or-v1-{}", random_part)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{AuditListItem, AuditListQuery, AuditRecord};
    use crate::db::ApiKeyAuthRow;
    use crate::services::error::RepositoryError;
    use crate::services::repository::ApiKeySummary as RepoApiKeySummary;

    struct ConflictRepo;

    impl Repository for ConflictRepo {
        fn get_api_key_info(&self, _api_key: &str) -> Result<(i64, i64), RepositoryError> {
            Err(RepositoryError::NotFound("api key not found".into()))
        }
        fn get_api_key_auth(&self, _api_key: &str) -> Result<ApiKeyAuthRow, RepositoryError> {
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
        fn increment_quota_tokens(&self, _key_id: i64, _delta: i64) -> Result<(), RepositoryError> {
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
        fn list_api_keys_for_user(
            &self,
            _user_id: i64,
        ) -> Result<Vec<RepoApiKeySummary>, RepositoryError> {
            Ok(Vec::new())
        }
        fn get_api_key_for_user(
            &self,
            _user_id: i64,
            _key_id: i64,
        ) -> Result<RepoApiKeySummary, RepositoryError> {
            Err(RepositoryError::NotFound("x".into()))
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
            _patch: &ApiKeyPatchDb,
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
