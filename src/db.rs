use rusqlite::{params, params_from_iter, types::Value, Connection};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::audit::{AuditListItem, AuditListQuery, AuditRecord};

const MIGRATIONS: [(&str, &str); 3] = [
    (
        "0001_create_users.sql",
        include_str!("../migrations/0001_create_users.sql"),
    ),
    (
        "0002_create_audit_logs.sql",
        include_str!("../migrations/0002_create_audit_logs.sql"),
    ),
    (
        "0003_users_password_hash.sql",
        include_str!("../migrations/0003_users_password_hash.sql"),
    ),
];

pub type DbConn = Pool<SqliteConnectionManager>;

pub fn create_db_pool(path: &str) -> Result<DbConn, r2d2::Error> {
    let manager = SqliteConnectionManager::file(path);
    r2d2::Pool::builder().max_size(16).build(manager)
}

pub fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS migration_versions (
            version TEXT PRIMARY KEY,
            applied_at INTEGER NOT NULL
        )",
        [],
    )?;

    for (version, sql) in MIGRATIONS.iter() {
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM migration_versions WHERE version = ?1)",
            params![*version],
            |row| row.get(0),
        )?;

        if exists {
            continue;
        }

        conn.execute_batch(sql)?;

        let applied_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO migration_versions (version, applied_at) VALUES (?1, ?2)",
            params![*version, applied_at],
        )?;
    }

    Ok(())
}

pub fn create_user(conn: &Connection, username: &str, created_at: i64) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO users (username, created_at) VALUES (?1, ?2)",
        params![username, created_at],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn create_api_key_for_user(
    conn: &Connection,
    user_id: i64,
    api_key: &str,
    created_at: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO api_keys (user_id, api_key, created_at) VALUES (?1, ?2, ?3)",
        params![user_id, api_key, created_at],
    )?;
    Ok(())
}

pub fn find_user_id(conn: &Connection, username: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT id FROM users WHERE username = ?1",
        params![username],
        |row| row.get(0),
    )
}

/// Returns `Some((user_id, password_hash))` if the user exists. `password_hash` is `None` for legacy rows.
pub fn get_user_login_credentials(
    conn: &Connection,
    username: &str,
) -> rusqlite::Result<Option<(i64, Option<String>)>> {
    let mut stmt = conn.prepare("SELECT id, password_hash FROM users WHERE username = ?1")?;
    let mut rows = stmt.query_map(params![username], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn insert_user_with_password(
    conn: &Connection,
    username: &str,
    password_hash: &str,
    created_at: i64,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO users (username, created_at, password_hash) VALUES (?1, ?2, ?3)",
        params![username, created_at, password_hash],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_first_api_key_for_user(
    conn: &Connection,
    user_id: i64,
) -> rusqlite::Result<Option<String>> {
    let mut stmt = conn.prepare(
        "SELECT api_key FROM api_keys WHERE user_id = ?1 AND revoked = 0 ORDER BY id DESC LIMIT 1",
    )?;
    let mut rows = stmt.query_map(params![user_id], |row| row.get(0))?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

pub fn validate_api_key(conn: &Connection, api_key: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM api_keys WHERE api_key = ?1 AND revoked = 0",
        params![api_key],
        |_| Ok(()),
    )
    .is_ok()
}

pub fn get_api_key_info(conn: &Connection, api_key: &str) -> rusqlite::Result<(i64, i64)> {
    conn.query_row(
        "SELECT id, user_id FROM api_keys WHERE api_key = ?1 AND revoked = 0",
        params![api_key],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
}

pub fn insert_audit_logs(conn: &mut Connection, records: &[AuditRecord]) -> rusqlite::Result<()> {
    if records.is_empty() {
        return Ok(());
    }

    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT OR REPLACE INTO audit_logs (
                request_id, user_id, token_id, channel_id, model, request_type,
                request_body_path, response_body_path, status_code, error_message,
                prompt_tokens, completion_tokens, total_tokens, cost, latency_ms,
                metadata, created_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17
            )",
        )?;

        for record in records {
            let metadata = record.metadata.as_ref().map(|v| v.to_string());
            stmt.execute(params![
                record.request_id,
                record.user_id,
                record.token_id,
                record.channel_id,
                record.model,
                record.request_type,
                record.request_body_path,
                record.response_body_path,
                record.status_code,
                record.error_message,
                record.prompt_tokens,
                record.completion_tokens,
                record.total_tokens,
                record.cost,
                record.latency_ms,
                metadata,
                record.created_at
            ])?;
        }
    }
    tx.commit()
}

pub fn query_audit_logs(
    conn: &Connection,
    query: &AuditListQuery,
    scoped_user_id: Option<i64>,
) -> rusqlite::Result<(Vec<AuditListItem>, i64)> {
    let (where_sql, where_args) = build_audit_where_clause(query, scoped_user_id);
    let limit = query.limit.unwrap_or(100).clamp(1, 1000);
    let offset = query.offset.unwrap_or(0);

    let list_sql = format!(
        "SELECT
            request_id, user_id, token_id, channel_id, model, request_type,
            status_code, error_message, prompt_tokens, completion_tokens,
            total_tokens, cost, latency_ms, created_at
         FROM audit_logs
         {where_sql}
         ORDER BY created_at DESC
         LIMIT ? OFFSET ?"
    );

    let mut list_args = where_args.clone();
    list_args.push(Value::Integer(limit as i64));
    list_args.push(Value::Integer(offset as i64));
    let mut stmt = conn.prepare(&list_sql)?;
    let rows = stmt.query_map(params_from_iter(list_args.iter()), |row| {
        Ok(AuditListItem {
            request_id: row.get(0)?,
            user_id: row.get(1)?,
            token_id: row.get(2)?,
            channel_id: row.get(3)?,
            model: row.get(4)?,
            request_type: row.get(5)?,
            status_code: row.get(6)?,
            error_message: row.get(7)?,
            prompt_tokens: row.get(8)?,
            completion_tokens: row.get(9)?,
            total_tokens: row.get(10)?,
            cost: row.get(11)?,
            latency_ms: row.get(12)?,
            created_at: row.get(13)?,
        })
    })?;

    let mut records = Vec::new();
    for row in rows {
        records.push(row?);
    }

    let count_sql = format!("SELECT COUNT(1) FROM audit_logs {where_sql}");
    let total = conn.query_row(&count_sql, params_from_iter(where_args.iter()), |row| row.get(0))?;
    Ok((records, total))
}

pub fn get_audit_log_by_request_id(
    conn: &Connection,
    request_id: &str,
    scoped_user_id: Option<i64>,
) -> rusqlite::Result<AuditRecord> {
    let mut sql = "SELECT
            request_id, user_id, token_id, channel_id, model, request_type,
            request_body_path, response_body_path, status_code, error_message,
            prompt_tokens, completion_tokens, total_tokens, cost, latency_ms,
            metadata, created_at
         FROM audit_logs
         WHERE request_id = ?"
        .to_string();
    let mut args = vec![Value::Text(request_id.to_string())];
    if let Some(user_id) = scoped_user_id {
        sql.push_str(" AND user_id = ?");
        args.push(Value::Integer(user_id));
    }

    conn.query_row(&sql, params_from_iter(args.iter()), |row| {
        let metadata_str: Option<String> = row.get(15)?;
        let metadata = metadata_str.and_then(|raw| serde_json::from_str(&raw).ok());
        Ok(AuditRecord {
            request_id: row.get(0)?,
            user_id: row.get(1)?,
            token_id: row.get(2)?,
            channel_id: row.get(3)?,
            model: row.get(4)?,
            request_type: row.get(5)?,
            request_body_path: row.get(6)?,
            response_body_path: row.get(7)?,
            status_code: row.get(8)?,
            error_message: row.get(9)?,
            prompt_tokens: row.get(10)?,
            completion_tokens: row.get(11)?,
            total_tokens: row.get(12)?,
            cost: row.get(13)?,
            latency_ms: row.get(14)?,
            metadata,
            created_at: row.get(16)?,
        })
    })
}

fn build_audit_where_clause(
    query: &AuditListQuery,
    scoped_user_id: Option<i64>,
) -> (String, Vec<Value>) {
    let mut where_clauses: Vec<&str> = Vec::new();
    let mut args: Vec<Value> = Vec::new();

    if let Some(user_id) = scoped_user_id.or(query.user_id) {
        where_clauses.push("user_id = ?");
        args.push(Value::Integer(user_id));
    }
    if let Some(token_id) = query.token_id {
        where_clauses.push("token_id = ?");
        args.push(Value::Integer(token_id));
    }
    if let Some(start_time) = query.start_time {
        where_clauses.push("created_at >= ?");
        args.push(Value::Integer(start_time));
    }
    if let Some(end_time) = query.end_time {
        where_clauses.push("created_at <= ?");
        args.push(Value::Integer(end_time));
    }
    if let Some(status_code) = query.status_code {
        where_clauses.push("status_code = ?");
        args.push(Value::Integer(status_code));
    }
    if let Some(channel_id) = &query.channel_id {
        where_clauses.push("channel_id = ?");
        args.push(Value::Text(channel_id.clone()));
    }
    if let Some(model) = &query.model {
        where_clauses.push("model = ?");
        args.push(Value::Text(model.clone()));
    }
    if let Some(keyword) = &query.keyword {
        let like_kw = format!("%{keyword}%");
        where_clauses.push("(request_id LIKE ? OR error_message LIKE ? OR model LIKE ?)");
        args.push(Value::Text(like_kw.clone()));
        args.push(Value::Text(like_kw.clone()));
        args.push(Value::Text(like_kw));
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    (where_sql, args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn now_secs() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    #[test]
    fn run_migrations_is_idempotent() {
        let conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("first migration");
        run_migrations(&conn).expect("second migration");
    }

    #[test]
    fn create_user_and_find_user() {
        let conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");

        let created_at = now_secs();
        let user_id = create_user(&conn, "alice", created_at).expect("create user");
        assert!(user_id > 0);

        let found = find_user_id(&conn, "alice").expect("find user");
        assert_eq!(found, user_id);
    }

    #[test]
    fn create_api_key_and_validate() {
        let conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");

        let created_at = now_secs();
        let user_id = create_user(&conn, "bob", created_at).expect("create user");
        create_api_key_for_user(&conn, user_id, "key123", created_at).expect("create api key");

        assert!(validate_api_key(&conn, "key123"));
        assert!(!validate_api_key(&conn, "missing"));
    }

    #[test]
    fn find_user_id_nonexistent_returns_error() {
        let conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");

        assert!(find_user_id(&conn, "nobody").is_err());
    }
}
