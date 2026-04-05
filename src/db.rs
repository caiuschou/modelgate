use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, params_from_iter, types::Value, Connection};

use crate::audit::{AuditListItem, AuditListQuery, AuditRecord};

const MIGRATIONS: [(&str, &str); 5] = [
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
    (
        "0004_audit_app_finish_reason.sql",
        include_str!("../migrations/0004_audit_app_finish_reason.sql"),
    ),
    (
        "0005_api_keys_enhance.sql",
        include_str!("../migrations/0005_api_keys_enhance.sql"),
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
    insert_api_key_for_user(conn, user_id, api_key, created_at)?;
    Ok(())
}

/// Inserts a row into `api_keys` and returns the new row id.
pub fn insert_api_key_for_user(
    conn: &Connection,
    user_id: i64,
    api_key: &str,
    created_at: i64,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO api_keys (user_id, api_key, created_at) VALUES (?1, ?2, ?3)",
        params![user_id, api_key, created_at],
    )?;
    Ok(conn.last_insert_rowid())
}

#[derive(Debug, Clone)]
pub struct ApiKeyRow {
    pub id: i64,
    pub api_key: String,
    pub created_at: i64,
    pub revoked: i32,
    pub name: String,
    pub description: String,
    pub disabled: i32,
    pub last_used_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub quota_monthly_tokens: Option<i64>,
    pub quota_used_tokens: i64,
    pub model_allowlist: Option<String>,
    pub ip_allowlist: Option<String>,
}

pub fn list_api_keys_for_user(conn: &Connection, user_id: i64) -> rusqlite::Result<Vec<ApiKeyRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, api_key, created_at, revoked, name, description, disabled, last_used_at, expires_at,
                quota_monthly_tokens, quota_used_tokens, model_allowlist, ip_allowlist
         FROM api_keys WHERE user_id = ?1 ORDER BY id DESC",
    )?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok(ApiKeyRow {
            id: row.get(0)?,
            api_key: row.get(1)?,
            created_at: row.get(2)?,
            revoked: row.get(3)?,
            name: row.get(4)?,
            description: row.get(5)?,
            disabled: row.get(6)?,
            last_used_at: row.get(7)?,
            expires_at: row.get(8)?,
            quota_monthly_tokens: row.get(9)?,
            quota_used_tokens: row.get(10)?,
            model_allowlist: row.get(11)?,
            ip_allowlist: row.get(12)?,
        })
    })?;
    rows.collect()
}

pub fn get_api_key_row_for_user(
    conn: &Connection,
    user_id: i64,
    key_id: i64,
) -> rusqlite::Result<ApiKeyRow> {
    conn.query_row(
        "SELECT id, api_key, created_at, revoked, name, description, disabled, last_used_at, expires_at,
                quota_monthly_tokens, quota_used_tokens, model_allowlist, ip_allowlist
         FROM api_keys WHERE id = ?1 AND user_id = ?2",
        params![key_id, user_id],
        |row| {
            Ok(ApiKeyRow {
                id: row.get(0)?,
                api_key: row.get(1)?,
                created_at: row.get(2)?,
                revoked: row.get(3)?,
                name: row.get(4)?,
                description: row.get(5)?,
                disabled: row.get(6)?,
                last_used_at: row.get(7)?,
                expires_at: row.get(8)?,
                quota_monthly_tokens: row.get(9)?,
                quota_used_tokens: row.get(10)?,
                model_allowlist: row.get(11)?,
                ip_allowlist: row.get(12)?,
            })
        },
    )
}

pub fn insert_api_key_with_meta(
    conn: &Connection,
    user_id: i64,
    api_key: &str,
    created_at: i64,
    name: &str,
    description: &str,
    expires_at: Option<i64>,
    quota_monthly_tokens: Option<i64>,
    model_allowlist: Option<&str>,
    ip_allowlist: Option<&str>,
) -> rusqlite::Result<i64> {
    let period = quota_monthly_tokens
        .filter(|&q| q > 0)
        .map(|_| crate::api_key_policy::unix_month_start(created_at));
    conn.execute(
        "INSERT INTO api_keys (user_id, api_key, created_at, name, description, expires_at,
            quota_monthly_tokens, model_allowlist, ip_allowlist, quota_period_start)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            user_id,
            api_key,
            created_at,
            name,
            description,
            expires_at,
            quota_monthly_tokens,
            model_allowlist,
            ip_allowlist,
            period,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

#[derive(Debug, Default)]
pub struct ApiKeyPatchDb {
    pub name: Option<String>,
    pub description: Option<String>,
    pub disabled: Option<bool>,
    pub expires_at: Option<Option<i64>>,
    pub quota_monthly_tokens: Option<Option<i64>>,
    pub model_allowlist: Option<Option<String>>,
    pub ip_allowlist: Option<Option<String>>,
}

pub fn update_api_key_for_user(
    conn: &Connection,
    user_id: i64,
    key_id: i64,
    patch: &ApiKeyPatchDb,
) -> rusqlite::Result<usize> {
    let mut total: usize = 0;
    if let Some(ref n) = patch.name {
        total += conn.execute(
            "UPDATE api_keys SET name = ?1 WHERE id = ?2 AND user_id = ?3 AND revoked = 0",
            params![n, key_id, user_id],
        )?;
    }
    if let Some(ref d) = patch.description {
        total += conn.execute(
            "UPDATE api_keys SET description = ?1 WHERE id = ?2 AND user_id = ?3 AND revoked = 0",
            params![d, key_id, user_id],
        )?;
    }
    if let Some(d) = patch.disabled {
        total += conn.execute(
            "UPDATE api_keys SET disabled = ?1 WHERE id = ?2 AND user_id = ?3 AND revoked = 0",
            params![if d { 1 } else { 0 }, key_id, user_id],
        )?;
    }
    if let Some(ref e) = patch.expires_at {
        total += match e {
            Some(ts) => conn.execute(
                "UPDATE api_keys SET expires_at = ?1 WHERE id = ?2 AND user_id = ?3 AND revoked = 0",
                params![*ts, key_id, user_id],
            )?,
            None => conn.execute(
                "UPDATE api_keys SET expires_at = NULL WHERE id = ?1 AND user_id = ?2 AND revoked = 0",
                params![key_id, user_id],
            )?,
        };
    }
    if let Some(ref q) = patch.quota_monthly_tokens {
        total += match q {
            Some(v) => conn.execute(
                "UPDATE api_keys SET quota_monthly_tokens = ?1 WHERE id = ?2 AND user_id = ?3 AND revoked = 0",
                params![*v, key_id, user_id],
            )?,
            None => conn.execute(
                "UPDATE api_keys SET quota_monthly_tokens = NULL, quota_used_tokens = 0, quota_period_start = NULL WHERE id = ?1 AND user_id = ?2 AND revoked = 0",
                params![key_id, user_id],
            )?,
        };
    }
    if let Some(ref m) = patch.model_allowlist {
        total += match m {
            Some(s) => conn.execute(
                "UPDATE api_keys SET model_allowlist = ?1 WHERE id = ?2 AND user_id = ?3 AND revoked = 0",
                params![s, key_id, user_id],
            )?,
            None => conn.execute(
                "UPDATE api_keys SET model_allowlist = NULL WHERE id = ?1 AND user_id = ?2 AND revoked = 0",
                params![key_id, user_id],
            )?,
        };
    }
    if let Some(ref ip) = patch.ip_allowlist {
        total += match ip {
            Some(s) => conn.execute(
                "UPDATE api_keys SET ip_allowlist = ?1 WHERE id = ?2 AND user_id = ?3 AND revoked = 0",
                params![s, key_id, user_id],
            )?,
            None => conn.execute(
                "UPDATE api_keys SET ip_allowlist = NULL WHERE id = ?1 AND user_id = ?2 AND revoked = 0",
                params![key_id, user_id],
            )?,
        };
    }
    Ok(total)
}

/// Sets `revoked = 1` for the key if it belongs to `user_id` and is not already revoked.
/// Returns number of rows updated (0 or 1).
pub fn revoke_api_key_for_user(
    conn: &Connection,
    user_id: i64,
    key_id: i64,
) -> rusqlite::Result<usize> {
    let n = conn.execute(
        "UPDATE api_keys SET revoked = 1 WHERE id = ?1 AND user_id = ?2 AND revoked = 0",
        params![key_id, user_id],
    )?;
    Ok(n)
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
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    conn.query_row(
        "SELECT 1 FROM api_keys WHERE api_key = ?1 AND revoked = 0 AND disabled = 0
         AND (expires_at IS NULL OR expires_at > ?2)",
        params![api_key, now],
        |_| Ok(()),
    )
    .is_ok()
}

/// Active key: not revoked/disabled/expired.
pub fn get_api_key_info(conn: &Connection, api_key: &str) -> rusqlite::Result<(i64, i64)> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    conn.query_row(
        "SELECT id, user_id FROM api_keys WHERE api_key = ?1 AND revoked = 0 AND disabled = 0
         AND (expires_at IS NULL OR expires_at > ?2)",
        params![api_key, now],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
}

#[derive(Debug, Clone)]
pub struct ApiKeyAuthRow {
    pub id: i64,
    pub user_id: i64,
    pub model_allowlist: Option<String>,
    pub ip_allowlist: Option<String>,
    pub quota_monthly_tokens: Option<i64>,
    pub quota_used_tokens: i64,
    pub quota_period_start: Option<i64>,
}

pub fn get_api_key_auth_row(conn: &Connection, api_key: &str) -> rusqlite::Result<ApiKeyAuthRow> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    conn.query_row(
        "SELECT id, user_id, model_allowlist, ip_allowlist, quota_monthly_tokens,
                quota_used_tokens, quota_period_start
         FROM api_keys WHERE api_key = ?1 AND revoked = 0 AND disabled = 0
         AND (expires_at IS NULL OR expires_at > ?2)",
        params![api_key, now],
        |row| {
            Ok(ApiKeyAuthRow {
                id: row.get(0)?,
                user_id: row.get(1)?,
                model_allowlist: row.get(2)?,
                ip_allowlist: row.get(3)?,
                quota_monthly_tokens: row.get(4)?,
                quota_used_tokens: row.get(5)?,
                quota_period_start: row.get(6)?,
            })
        },
    )
}

/// Reset monthly quota if we crossed into a new UTC calendar month; then check headroom.
pub fn ensure_monthly_quota(
    conn: &Connection,
    key_id: i64,
    now: i64,
) -> Result<(), &'static str> {
    use crate::api_key_policy::unix_month_start;
    let month_start = unix_month_start(now);
    let (limit, used, period): (Option<i64>, i64, Option<i64>) = conn.query_row(
        "SELECT quota_monthly_tokens, quota_used_tokens, quota_period_start FROM api_keys WHERE id = ?1",
        params![key_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .map_err(|_| "api key not found")?;
    let Some(limit) = limit.filter(|&l| l > 0) else {
        return Ok(());
    };
    let mut used = used;
    if period.map(|p| p < month_start).unwrap_or(true) {
        used = 0;
        conn.execute(
            "UPDATE api_keys SET quota_used_tokens = 0, quota_period_start = ?1 WHERE id = ?2",
            params![month_start, key_id],
        )
        .map_err(|_| "database error")?;
    }
    if used >= limit {
        return Err("monthly token quota exceeded");
    }
    Ok(())
}

pub fn increment_quota_tokens(conn: &Connection, key_id: i64, delta: i64) -> rusqlite::Result<()> {
    if delta <= 0 {
        return Ok(());
    }
    conn.execute(
        "UPDATE api_keys SET quota_used_tokens = quota_used_tokens + ?1 WHERE id = ?2 AND quota_monthly_tokens IS NOT NULL",
        params![delta, key_id],
    )?;
    Ok(())
}

/// Throttle writes: only update if never set or older than `min_interval_secs`.
pub fn touch_api_key_last_used(
    conn: &Connection,
    key_id: i64,
    now: i64,
    min_interval_secs: i64,
) -> rusqlite::Result<()> {
    let should: bool = conn.query_row(
        "SELECT last_used_at IS NULL OR (?1 - last_used_at) >= ?2 FROM api_keys WHERE id = ?3",
        params![now, min_interval_secs, key_id],
        |row| row.get(0),
    )?;
    if should {
        conn.execute(
            "UPDATE api_keys SET last_used_at = ?1 WHERE id = ?2",
            params![now, key_id],
        )?;
    }
    Ok(())
}

pub fn insert_api_key_audit(
    conn: &Connection,
    user_id: i64,
    key_id: i64,
    action: &str,
    created_at: i64,
    detail: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO api_key_audit_log (user_id, key_id, action, created_at, detail) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![user_id, key_id, action, created_at, detail],
    )?;
    Ok(())
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
                app_id, finish_reason, metadata, created_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19
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
                record.app_id,
                record.finish_reason,
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
            total_tokens, cost, latency_ms, app_id, finish_reason, created_at
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
            app_id: row.get(13)?,
            finish_reason: row.get(14)?,
            created_at: row.get(15)?,
        })
    })?;

    let mut records = Vec::new();
    for row in rows {
        records.push(row?);
    }

    let count_sql = format!("SELECT COUNT(1) FROM audit_logs {where_sql}");
    let total = conn.query_row(&count_sql, params_from_iter(where_args.iter()), |row| {
        row.get(0)
    })?;
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
            app_id, finish_reason, metadata, created_at
         FROM audit_logs
         WHERE request_id = ?"
        .to_string();
    let mut args = vec![Value::Text(request_id.to_string())];
    if let Some(user_id) = scoped_user_id {
        sql.push_str(" AND user_id = ?");
        args.push(Value::Integer(user_id));
    }

    conn.query_row(&sql, params_from_iter(args.iter()), |row| {
        let metadata_str: Option<String> = row.get(17)?;
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
            app_id: row.get(15)?,
            finish_reason: row.get(16)?,
            metadata,
            created_at: row.get(18)?,
        })
    })
}

fn build_audit_where_clause(
    query: &AuditListQuery,
    scoped_user_id: Option<i64>,
) -> (String, Vec<Value>) {
    let mut where_clauses: Vec<String> = Vec::new();
    let mut args: Vec<Value> = Vec::new();

    if let Some(user_id) = scoped_user_id.or(query.user_id) {
        where_clauses.push("user_id = ?".to_string());
        args.push(Value::Integer(user_id));
    }
    if let Some(token_id) = query.token_id {
        where_clauses.push("token_id = ?".to_string());
        args.push(Value::Integer(token_id));
    }
    if let Some(start_time) = query.start_time {
        where_clauses.push("created_at >= ?".to_string());
        args.push(Value::Integer(start_time));
    }
    if let Some(end_time) = query.end_time {
        where_clauses.push("created_at <= ?".to_string());
        args.push(Value::Integer(end_time));
    }
    if let Some(status_code) = query.status_code {
        where_clauses.push("status_code = ?".to_string());
        args.push(Value::Integer(status_code));
    }
    if let Some(channel_id) = &query.channel_id {
        where_clauses.push("channel_id = ?".to_string());
        args.push(Value::Text(channel_id.clone()));
    }
    if let Some(model) = &query.model {
        where_clauses.push("model = ?".to_string());
        args.push(Value::Text(model.clone()));
    }
    if let Some(keyword) = &query.keyword {
        let like_kw = format!("%{keyword}%");
        where_clauses
            .push("(request_id LIKE ? OR error_message LIKE ? OR model LIKE ?)".to_string());
        args.push(Value::Text(like_kw.clone()));
        args.push(Value::Text(like_kw.clone()));
        args.push(Value::Text(like_kw));
    }
    if let Some(app_id) = &query.app_id {
        if !app_id.is_empty() {
            where_clauses.push("app_id = ?".to_string());
            args.push(Value::Text(app_id.clone()));
        }
    }
    if let Some(fr) = &query.finish_reason {
        let parts: Vec<String> = fr
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !parts.is_empty() {
            let placeholders: Vec<&str> = parts.iter().map(|_| "?").collect();
            where_clauses.push(format!("finish_reason IN ({})", placeholders.join(", ")));
            for p in parts {
                args.push(Value::Text(p));
            }
        }
    }
    if let Some(v) = query.min_prompt_tokens {
        where_clauses.push("prompt_tokens >= ?".to_string());
        args.push(Value::Integer(v));
    }
    if let Some(v) = query.max_prompt_tokens {
        where_clauses.push("prompt_tokens <= ?".to_string());
        args.push(Value::Integer(v));
    }
    if let Some(v) = query.min_completion_tokens {
        where_clauses.push("completion_tokens >= ?".to_string());
        args.push(Value::Integer(v));
    }
    if let Some(v) = query.max_completion_tokens {
        where_clauses.push("completion_tokens <= ?".to_string());
        args.push(Value::Integer(v));
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
