use rusqlite::ErrorCode;
use tracing::error;

use crate::audit::{AuditListItem, AuditListQuery, AuditRecord};
use crate::db;

use super::error::RepositoryError;

pub trait Repository: Send + Sync {
    fn get_api_key_info(&self, api_key: &str) -> Result<(i64, i64), RepositoryError>;
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
        if let Err(err) = tx.execute(
            "INSERT INTO api_keys (user_id, api_key, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![user_id, api_key, created_at as i64],
        ) {
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

        conn.execute(
            "INSERT INTO api_keys (user_id, api_key, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![user_id, api_key, created_at as i64],
        )
        .map_err(|err| {
            error!(error = %err, "failed to create api key for user");
            RepositoryError::Internal("Failed to create api key".into())
        })?;
        Ok(())
    }
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
        let (token_id, user_id) = repo
            .get_api_key_info("key-alice")
            .expect("lookup key info");
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
}

