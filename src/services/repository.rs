use rusqlite::ErrorCode;
use serde::Serialize;
use tracing::error;

use crate::audit::{AuditListItem, AuditListQuery, AuditRecord};
use crate::db;

use super::error::RepositoryError;

#[derive(Debug, Clone, Serialize)]
pub struct ApiKeySummary {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub preview: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub revoked: bool,
    pub disabled: bool,
    pub expires_at: Option<i64>,
    pub quota_monthly_tokens: Option<i64>,
    pub quota_used_tokens: i64,
    pub model_allowlist: Option<Vec<String>>,
    pub ip_allowlist: Option<Vec<String>>,
    pub status: String,
}

fn mask_api_key_preview(full: &str) -> String {
    let b = full.as_bytes();
    if b.len() <= 14 {
        return "••••".to_string();
    }
    let start = std::str::from_utf8(&b[..12]).unwrap_or("••••");
    let end = std::str::from_utf8(&b[b.len() - 4..]).unwrap_or("");
    format!("{start}…{end}")
}

fn parse_json_string_list(raw: Option<String>) -> Option<Vec<String>> {
    raw.and_then(|s| {
        if s.trim().is_empty() {
            None
        } else {
            serde_json::from_str(&s).ok()
        }
    })
}

fn key_status(now: i64, r: &db::ApiKeyRow) -> String {
    if r.revoked != 0 {
        "revoked".to_string()
    } else if r.expires_at.map(|e| e <= now).unwrap_or(false) {
        "expired".to_string()
    } else if r.disabled != 0 {
        "disabled".to_string()
    } else {
        "active".to_string()
    }
}

fn row_to_summary(now: i64, r: db::ApiKeyRow) -> ApiKeySummary {
    let status = key_status(now, &r);
    let preview = if !r.key_preview.is_empty() {
        r.key_preview.clone()
    } else {
        r.api_key_plain
            .as_deref()
            .map(mask_api_key_preview)
            .unwrap_or_else(|| "••••".to_string())
    };
    ApiKeySummary {
        id: r.id,
        name: if r.name.trim().is_empty() {
            format!("未命名密钥 #{}", r.id)
        } else {
            r.name.clone()
        },
        description: r.description,
        preview,
        created_at: r.created_at,
        last_used_at: r.last_used_at,
        revoked: r.revoked != 0,
        disabled: r.disabled != 0,
        expires_at: r.expires_at,
        quota_monthly_tokens: r.quota_monthly_tokens,
        quota_used_tokens: r.quota_used_tokens,
        model_allowlist: parse_json_string_list(r.model_allowlist.clone()),
        ip_allowlist: parse_json_string_list(r.ip_allowlist.clone()),
        status,
    }
}

pub trait Repository: Send + Sync {
    fn get_api_key_info(&self, api_key: &str) -> Result<(i64, i64), RepositoryError>;
    fn get_api_key_auth(&self, api_key: &str) -> Result<db::ApiKeyAuthRow, RepositoryError>;
    fn touch_api_key_last_used(&self, key_id: i64, now: i64) -> Result<(), RepositoryError>;
    fn ensure_monthly_quota(&self, key_id: i64, now: i64) -> Result<(), RepositoryError>;
    fn increment_quota_tokens(&self, key_id: i64, delta: i64) -> Result<(), RepositoryError>;

    fn query_audit_logs(
        &self,
        query: &AuditListQuery,
        scoped_user_id: Option<i64>,
    ) -> Result<(Vec<AuditListItem>, i64), RepositoryError>;
    fn get_audit_log_by_request_id(
        &self,
        request_id: &str,
        scoped_user_id: Option<i64>,
    ) -> Result<AuditRecord, RepositoryError>;
    fn create_user_with_api_key(
        &self,
        username: &str,
        api_key: &str,
        created_at: u64,
    ) -> Result<(), RepositoryError>;
    fn create_api_key_for_user(
        &self,
        username: &str,
        api_key: &str,
        created_at: u64,
    ) -> Result<(), RepositoryError>;

    fn register_user_with_password_and_api_key(
        &self,
        username: &str,
        password_hash: &str,
        api_key: &str,
        created_at: u64,
    ) -> Result<(), RepositoryError>;

    fn get_user_login_credentials(
        &self,
        username: &str,
    ) -> Result<Option<(i64, Option<String>)>, RepositoryError>;

    fn get_first_api_key_for_user(&self, user_id: i64) -> Result<Option<String>, RepositoryError>;

    fn create_api_key_for_user_id(
        &self,
        user_id: i64,
        api_key: &str,
        created_at: u64,
    ) -> Result<(), RepositoryError>;

    fn list_api_keys_for_user(&self, user_id: i64) -> Result<Vec<ApiKeySummary>, RepositoryError>;
    fn get_api_key_for_user(&self, user_id: i64, key_id: i64) -> Result<ApiKeySummary, RepositoryError>;

    fn insert_api_key_with_meta(
        &self,
        user_id: i64,
        api_key: &str,
        created_at: u64,
        name: &str,
        description: &str,
        expires_at: Option<i64>,
        quota_monthly_tokens: Option<i64>,
        model_allowlist: Option<&str>,
        ip_allowlist: Option<&str>,
    ) -> Result<i64, RepositoryError>;

    fn update_api_key_for_user(
        &self,
        user_id: i64,
        key_id: i64,
        patch: &db::ApiKeyPatchDb,
    ) -> Result<(), RepositoryError>;

    fn insert_api_key_audit(
        &self,
        user_id: i64,
        key_id: i64,
        action: &str,
        created_at: i64,
        detail: Option<&str>,
    ) -> Result<(), RepositoryError>;

    fn revoke_api_key_for_user(&self, user_id: i64, key_id: i64) -> Result<(), RepositoryError>;
}

#[derive(Clone)]
pub struct SqliteRepository {
    db_pool: db::DbConn,
}

impl SqliteRepository {
    pub fn new(db_pool: db::DbConn) -> Self {
        Self { db_pool }
    }
}

impl Repository for SqliteRepository {
    fn get_api_key_info(&self, api_key: &str) -> Result<(i64, i64), RepositoryError> {
        let conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        db::get_api_key_info(&conn, api_key)
            .map_err(|_| RepositoryError::NotFound("api key not found".into()))
    }

    fn get_api_key_auth(&self, api_key: &str) -> Result<db::ApiKeyAuthRow, RepositoryError> {
        let conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        db::get_api_key_auth_row(&conn, api_key)
            .map_err(|_| RepositoryError::NotFound("api key not found".into()))
    }

    fn touch_api_key_last_used(&self, key_id: i64, now: i64) -> Result<(), RepositoryError> {
        let conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        db::touch_api_key_last_used(&conn, key_id, now, 60).map_err(|e| {
            error!(error = %e, "touch last_used");
            RepositoryError::Internal("failed to update key".into())
        })
    }

    fn ensure_monthly_quota(&self, key_id: i64, now: i64) -> Result<(), RepositoryError> {
        let conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        db::ensure_monthly_quota(&conn, key_id, now).map_err(|msg| {
            if msg == "monthly token quota exceeded" {
                RepositoryError::Forbidden(msg.into())
            } else {
                RepositoryError::Internal(msg.into())
            }
        })
    }

    fn increment_quota_tokens(&self, key_id: i64, delta: i64) -> Result<(), RepositoryError> {
        let conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        db::increment_quota_tokens(&conn, key_id, delta).map_err(|e| {
            error!(error = %e, "increment quota");
            RepositoryError::Internal("failed to update quota".into())
        })
    }

    fn query_audit_logs(
        &self,
        query: &AuditListQuery,
        scoped_user_id: Option<i64>,
    ) -> Result<(Vec<AuditListItem>, i64), RepositoryError> {
        let conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        db::query_audit_logs(&conn, query, scoped_user_id)
            .map_err(|_| RepositoryError::Internal("Failed to query audit logs".into()))
    }

    fn get_audit_log_by_request_id(
        &self,
        request_id: &str,
        scoped_user_id: Option<i64>,
    ) -> Result<AuditRecord, RepositoryError> {
        let conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        db::get_audit_log_by_request_id(&conn, request_id, scoped_user_id)
            .map_err(|_| RepositoryError::NotFound("audit log not found".into()))
    }

    fn create_user_with_api_key(
        &self,
        username: &str,
        api_key: &str,
        created_at: u64,
    ) -> Result<(), RepositoryError> {
        let mut conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        let tx = conn.transaction().map_err(|err| {
            error!(error = %err, "failed to begin transaction");
            RepositoryError::Internal("Failed to create user".into())
        })?;

        if let Err(err) = tx.execute(
            "INSERT INTO users (username, created_at) VALUES (?1, ?2)",
            rusqlite::params![username, created_at as i64],
        ) {
            if let rusqlite::Error::SqliteFailure(err_code, _) = &err {
                if err_code.code == ErrorCode::ConstraintViolation {
                    return Err(RepositoryError::Conflict("username already exists".into()));
                }
            }
            error!(error = %err, "failed to create user");
            return Err(RepositoryError::Internal("Failed to create user".into()));
        }

        let user_id = tx.last_insert_rowid();
        if let Err(err) = db::insert_api_key_for_user(&tx, user_id, api_key, created_at as i64) {
            error!(error = %err, "failed to create user api key");
            return Err(RepositoryError::Internal("Failed to create user".into()));
        }

        tx.commit().map_err(|err| {
            error!(error = %err, "failed to commit transaction");
            RepositoryError::Internal("Failed to create user".into())
        })?;
        Ok(())
    }

    fn create_api_key_for_user(
        &self,
        username: &str,
        api_key: &str,
        created_at: u64,
    ) -> Result<(), RepositoryError> {
        let conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        let user_id = db::find_user_id(&conn, username)
            .map_err(|_| RepositoryError::NotFound("user not found".into()))?;

        db::insert_api_key_for_user(&conn, user_id, api_key, created_at as i64).map_err(|err| {
            error!(error = %err, "failed to create api key for user");
            RepositoryError::Internal("Failed to create api key".into())
        })?;
        Ok(())
    }

    fn register_user_with_password_and_api_key(
        &self,
        username: &str,
        password_hash: &str,
        api_key: &str,
        created_at: u64,
    ) -> Result<(), RepositoryError> {
        let mut conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        let tx = conn.transaction().map_err(|err| {
            error!(error = %err, "failed to begin transaction");
            RepositoryError::Internal("Failed to create user".into())
        })?;

        if let Err(err) = tx.execute(
            "INSERT INTO users (username, created_at, password_hash) VALUES (?1, ?2, ?3)",
            rusqlite::params![username, created_at as i64, password_hash],
        ) {
            if let rusqlite::Error::SqliteFailure(err_code, _) = &err {
                if err_code.code == ErrorCode::ConstraintViolation {
                    return Err(RepositoryError::Conflict("username already exists".into()));
                }
            }
            error!(error = %err, "failed to register user");
            return Err(RepositoryError::Internal("Failed to create user".into()));
        }

        let user_id = tx.last_insert_rowid();
        if let Err(err) = db::insert_api_key_for_user(&tx, user_id, api_key, created_at as i64) {
            error!(error = %err, "failed to create user api key");
            return Err(RepositoryError::Internal("Failed to create user".into()));
        }

        tx.commit().map_err(|err| {
            error!(error = %err, "failed to commit transaction");
            RepositoryError::Internal("Failed to create user".into())
        })?;
        Ok(())
    }

    fn get_user_login_credentials(
        &self,
        username: &str,
    ) -> Result<Option<(i64, Option<String>)>, RepositoryError> {
        let conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        db::get_user_login_credentials(&conn, username)
            .map_err(|_| RepositoryError::Internal("Failed to load user".into()))
    }

    fn get_first_api_key_for_user(&self, user_id: i64) -> Result<Option<String>, RepositoryError> {
        let conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        db::get_first_api_key_for_user(&conn, user_id)
            .map_err(|_| RepositoryError::Internal("Failed to load api key".into()))
    }

    fn create_api_key_for_user_id(
        &self,
        user_id: i64,
        api_key: &str,
        created_at: u64,
    ) -> Result<(), RepositoryError> {
        let conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        db::create_api_key_for_user(&conn, user_id, api_key, created_at as i64).map_err(|err| {
            error!(error = %err, "failed to create api key for user id");
            RepositoryError::Internal("Failed to create api key".into())
        })?;
        Ok(())
    }

    fn list_api_keys_for_user(&self, user_id: i64) -> Result<Vec<ApiKeySummary>, RepositoryError> {
        let conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        let rows = db::list_api_keys_for_user(&conn, user_id).map_err(|err| {
            error!(error = %err, "failed to list api keys");
            RepositoryError::Internal("Failed to list api keys".into())
        })?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Ok(rows.into_iter().map(|r| row_to_summary(now, r)).collect())
    }

    fn get_api_key_for_user(&self, user_id: i64, key_id: i64) -> Result<ApiKeySummary, RepositoryError> {
        let conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        let row = db::get_api_key_row_for_user(&conn, user_id, key_id).map_err(|_| {
            RepositoryError::NotFound("api key not found".into())
        })?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        Ok(row_to_summary(now, row))
    }

    fn insert_api_key_with_meta(
        &self,
        user_id: i64,
        api_key: &str,
        created_at: u64,
        name: &str,
        description: &str,
        expires_at: Option<i64>,
        quota_monthly_tokens: Option<i64>,
        model_allowlist: Option<&str>,
        ip_allowlist: Option<&str>,
    ) -> Result<i64, RepositoryError> {
        let conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        db::insert_api_key_with_meta(
            &conn,
            user_id,
            api_key,
            created_at as i64,
            name,
            description,
            expires_at,
            quota_monthly_tokens,
            model_allowlist,
            ip_allowlist,
        )
        .map_err(|e| {
            error!(error = %e, "insert api key with meta");
            RepositoryError::Internal("Failed to create api key".into())
        })
    }

    fn update_api_key_for_user(
        &self,
        user_id: i64,
        key_id: i64,
        patch: &db::ApiKeyPatchDb,
    ) -> Result<(), RepositoryError> {
        let conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        let n = db::update_api_key_for_user(&conn, user_id, key_id, patch).map_err(|e| {
            error!(error = %e, "update api key");
            RepositoryError::Internal("Failed to update api key".into())
        })?;
        if n == 0 && patch_has_changes(patch) {
            return Err(RepositoryError::NotFound("api key not found".into()));
        }
        Ok(())
    }

    fn insert_api_key_audit(
        &self,
        user_id: i64,
        key_id: i64,
        action: &str,
        created_at: i64,
        detail: Option<&str>,
    ) -> Result<(), RepositoryError> {
        let conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        db::insert_api_key_audit(&conn, user_id, key_id, action, created_at, detail).map_err(|e| {
            error!(error = %e, "insert api key audit");
            RepositoryError::Internal("Failed to write audit".into())
        })
    }

    fn revoke_api_key_for_user(&self, user_id: i64, key_id: i64) -> Result<(), RepositoryError> {
        let conn = self
            .db_pool
            .get()
            .map_err(|_| RepositoryError::PoolUnavailable)?;
        let n = db::revoke_api_key_for_user(&conn, user_id, key_id).map_err(|err| {
            error!(error = %err, "failed to revoke api key");
            RepositoryError::Internal("Failed to revoke api key".into())
        })?;
        if n == 0 {
            return Err(RepositoryError::NotFound("api key not found".into()));
        }
        Ok(())
    }
}

fn patch_has_changes(p: &db::ApiKeyPatchDb) -> bool {
    p.name.is_some()
        || p.description.is_some()
        || p.disabled.is_some()
        || p.expires_at.is_some()
        || p.quota_monthly_tokens.is_some()
        || p.model_allowlist.is_some()
        || p.ip_allowlist.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use r2d2_sqlite::SqliteConnectionManager;

    fn build_repo() -> SqliteRepository {
        let manager = SqliteConnectionManager::memory();
        let pool = r2d2::Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("build sqlite pool");
        {
            let conn = pool.get().expect("get sqlite conn");
            db::run_migrations(&conn).expect("run migrations");
        }
        SqliteRepository::new(pool)
    }

    #[test]
    fn create_user_and_lookup_api_key_info() {
        let repo = build_repo();
        repo.create_user_with_api_key("alice", "key-alice", 1)
            .expect("create user with api key");
        let (token_id, user_id) = repo.get_api_key_info("key-alice").expect("lookup key info");
        assert!(token_id > 0);
        assert!(user_id > 0);
    }

    #[test]
    fn create_api_key_for_missing_user_returns_not_found() {
        let repo = build_repo();
        let err = repo
            .create_api_key_for_user("missing", "key-x", 1)
            .expect_err("should fail for missing user");
        match err {
            RepositoryError::NotFound(msg) => assert!(msg.contains("user not found")),
            _ => panic!("unexpected error variant"),
        }
    }

    #[test]
    fn register_with_password_stores_credentials_and_api_key() {
        let repo = build_repo();
        let hash = bcrypt::hash("Abcd1234", bcrypt::DEFAULT_COST).expect("hash");
        repo.register_user_with_password_and_api_key("reguser", &hash, "sk-reg-1", 200)
            .expect("register");
        let creds = repo
            .get_user_login_credentials("reguser")
            .expect("load creds");
        let (user_id, stored) = creds.expect("user exists");
        assert_eq!(stored.as_deref(), Some(hash.as_str()));
        assert!(repo.get_first_api_key_for_user(user_id).expect("load key").is_none());
        let (tid, uid) = repo.get_api_key_info("sk-reg-1").expect("lookup by full key");
        assert_eq!(uid, user_id);
        assert!(tid > 0);
    }
}
