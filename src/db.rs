use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};
use std::time::Duration;
use tracing::debug;

use crate::audit::{AuditListItem, AuditListQuery, AuditRecord, AuditThreadListItem};

const MIGRATIONS: [(&str, &str); 19] = [
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
    (
        "0006_api_keys_hash.sql",
        include_str!("../migrations/0006_api_keys_hash.sql"),
    ),
    (
        "0007_teams.sql",
        include_str!("../migrations/0007_teams.sql"),
    ),
    (
        "0008_team_scope_api_audit.sql",
        include_str!("../migrations/0008_team_scope_api_audit.sql"),
    ),
    (
        "0009_byok_profiles.sql",
        include_str!("../migrations/0009_byok_profiles.sql"),
    ),
    (
        "0010_api_keys_default_byok.sql",
        include_str!("../migrations/0010_api_keys_default_byok.sql"),
    ),
    (
        "0011_billing.sql",
        include_str!("../migrations/0011_billing.sql"),
    ),
    (
        "0012_billing_usd_minor_k15.sql",
        include_str!("../migrations/0012_billing_usd_minor_k15.sql"),
    ),
    (
        "0013_audit_hourly_rollups.sql",
        include_str!("../migrations/0013_audit_hourly_rollups.sql"),
    ),
    (
        "0014_audit_cached_prompt_tokens.sql",
        include_str!("../migrations/0014_audit_cached_prompt_tokens.sql"),
    ),
    (
        "0015_api_keys_concurrency_spend.sql",
        include_str!("../migrations/0015_api_keys_concurrency_spend.sql"),
    ),
    (
        "0016_audit_thread_id.sql",
        include_str!("../migrations/0016_audit_thread_id.sql"),
    ),
    (
        "0017_audit_reasoning_phase_ms.sql",
        include_str!("../migrations/0017_audit_reasoning_phase_ms.sql"),
    ),
    (
        "0018_audit_normalize_body_paths.sql",
        include_str!("../migrations/0018_audit_normalize_body_paths.sql"),
    ),
    (
        "0019_audit_threads.sql",
        include_str!("../migrations/0019_audit_threads.sql"),
    ),
];

/// How the console scopes audit listing / detail access.
#[derive(Clone, Copy, Debug)]
pub enum AuditConsoleScope {
    /// `WHERE user_id = ? AND team_id IS NULL`
    Personal(i64),
    /// `WHERE team_id = ?` (caller must verify membership)
    Team(i64),
    /// All proxy traffic for API keys owned by this user (`audit_logs.user_id`), including team keys.
    /// Used by the dashboard overview so team-key usage is visible without switching team context.
    UserOwnedTraffic(i64),
}

pub type DbConn = Pool<SqliteConnectionManager>;

/// WAL + busy timeout on every pooled connection so concurrent readers/writers (API handlers,
/// background audit flush) do not immediately fail with `SQLITE_BUSY` / "database is locked".
fn configure_sqlite_connection(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;",
    )?;
    Ok(())
}

pub fn create_db_pool(path: &str) -> Result<DbConn, r2d2::Error> {
    let manager = SqliteConnectionManager::file(path).with_init(configure_sqlite_connection);
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

    migrate_0006_api_key_hashes_if_needed(conn)?;
    Ok(())
}

fn migrate_0006_api_key_hashes_if_needed(conn: &Connection) -> rusqlite::Result<()> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM api_keys WHERE api_key IS NOT NULL AND LENGTH(TRIM(api_key)) > 0",
        [],
        |row| row.get(0),
    )?;
    if n == 0 {
        return Ok(());
    }
    migrate_0006_api_key_hashes(conn)
}

fn migrate_0006_api_key_hashes(conn: &Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(
        "SELECT id, api_key FROM api_keys WHERE api_key IS NOT NULL AND LENGTH(TRIM(api_key)) > 0",
    )?;
    let rows: Vec<(i64, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    for (id, key) in rows {
        let hash = crate::secrets::api_key_sha256_hex(&key);
        let preview = crate::secrets::api_key_preview_short(&key);
        conn.execute(
            "UPDATE api_keys SET api_key_hash = ?1, key_preview = ?2, api_key = NULL WHERE id = ?3",
            params![hash, preview, id],
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
    let hash = crate::secrets::api_key_sha256_hex(api_key);
    let preview = crate::secrets::api_key_preview_short(api_key);
    conn.execute(
        "INSERT INTO api_keys (user_id, api_key, api_key_hash, key_preview, created_at) VALUES (?1, NULL, ?2, ?3, ?4)",
        params![user_id, hash, preview, created_at],
    )?;
    Ok(conn.last_insert_rowid())
}

#[derive(Debug, Clone)]
pub struct ApiKeyRow {
    pub id: i64,
    pub user_id: i64,
    /// Legacy plaintext; always `None` after migration 0006 backfill.
    pub api_key_plain: Option<String>,
    pub key_preview: String,
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
    /// `None` = personal (non-team) key.
    pub team_id: Option<i64>,
    /// Default BYOK profile for Chat proxy when no `X-MG-Byok-Id` (see routing docs).
    pub default_byok_profile_id: Option<i64>,
    /// Max simultaneous upstream chat requests for this key (`None` = unlimited).
    pub max_concurrent_requests: Option<i32>,
    /// Monthly platform spend cap in USD minor units (k=15); `None` = no cap.
    pub quota_monthly_spend_minor: Option<String>,
    pub quota_used_spend_minor: String,
    pub quota_spend_period_start: Option<i64>,
}

fn map_api_key_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ApiKeyRow> {
    Ok(ApiKeyRow {
        id: row.get(0)?,
        user_id: row.get(1)?,
        api_key_plain: row.get(2)?,
        key_preview: row.get(3)?,
        created_at: row.get(4)?,
        revoked: row.get(5)?,
        name: row.get(6)?,
        description: row.get(7)?,
        disabled: row.get(8)?,
        last_used_at: row.get(9)?,
        expires_at: row.get(10)?,
        quota_monthly_tokens: row.get(11)?,
        quota_used_tokens: row.get(12)?,
        model_allowlist: row.get(13)?,
        ip_allowlist: row.get(14)?,
        team_id: row.get(15)?,
        default_byok_profile_id: row.get(16)?,
        max_concurrent_requests: row.get(17)?,
        quota_monthly_spend_minor: row.get(18)?,
        quota_used_spend_minor: row.get(19)?,
        quota_spend_period_start: row.get(20)?,
    })
}

/// Personal console keys: owned by user, not tied to a team.
pub fn list_api_keys_for_user(conn: &Connection, user_id: i64) -> rusqlite::Result<Vec<ApiKeyRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, user_id, api_key, key_preview, created_at, revoked, name, description, disabled, last_used_at, expires_at,
                quota_monthly_tokens, quota_used_tokens, model_allowlist, ip_allowlist, team_id, default_byok_profile_id,
                max_concurrent_requests, quota_monthly_spend_minor, quota_used_spend_minor, quota_spend_period_start
         FROM api_keys WHERE user_id = ?1 AND team_id IS NULL ORDER BY id DESC",
    )?;
    let rows = stmt.query_map(params![user_id], map_api_key_row)?;
    rows.collect()
}

/// All keys belonging to a team (any member may list).
pub fn list_api_keys_for_team(conn: &Connection, team_id: i64) -> rusqlite::Result<Vec<ApiKeyRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, user_id, api_key, key_preview, created_at, revoked, name, description, disabled, last_used_at, expires_at,
                quota_monthly_tokens, quota_used_tokens, model_allowlist, ip_allowlist, team_id, default_byok_profile_id,
                max_concurrent_requests, quota_monthly_spend_minor, quota_used_spend_minor, quota_spend_period_start
         FROM api_keys WHERE team_id = ?1 ORDER BY id DESC",
    )?;
    let rows = stmt.query_map(params![team_id], map_api_key_row)?;
    rows.collect()
}

pub fn get_api_key_row_for_user(
    conn: &Connection,
    user_id: i64,
    key_id: i64,
) -> rusqlite::Result<ApiKeyRow> {
    conn.query_row(
        "SELECT id, user_id, api_key, key_preview, created_at, revoked, name, description, disabled, last_used_at, expires_at,
                quota_monthly_tokens, quota_used_tokens, model_allowlist, ip_allowlist, team_id, default_byok_profile_id,
                max_concurrent_requests, quota_monthly_spend_minor, quota_used_spend_minor, quota_spend_period_start
         FROM api_keys WHERE id = ?1 AND user_id = ?2 AND team_id IS NULL",
        params![key_id, user_id],
        map_api_key_row,
    )
}

pub fn get_api_key_row_by_id(conn: &Connection, key_id: i64) -> rusqlite::Result<ApiKeyRow> {
    conn.query_row(
        "SELECT id, user_id, api_key, key_preview, created_at, revoked, name, description, disabled, last_used_at, expires_at,
                quota_monthly_tokens, quota_used_tokens, model_allowlist, ip_allowlist, team_id, default_byok_profile_id,
                max_concurrent_requests, quota_monthly_spend_minor, quota_used_spend_minor, quota_spend_period_start
         FROM api_keys WHERE id = ?1",
        params![key_id],
        map_api_key_row,
    )
}

#[allow(clippy::too_many_arguments)]
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
    team_id: Option<i64>,
    default_byok_profile_id: Option<i64>,
    max_concurrent_requests: Option<i32>,
    quota_monthly_spend_minor: Option<i128>,
) -> rusqlite::Result<i64> {
    let period = quota_monthly_tokens
        .filter(|&q| q > 0)
        .map(|_| crate::api_key_policy::unix_month_start(created_at));
    let spend_period = quota_monthly_spend_minor
        .filter(|&q| q > 0)
        .map(|_| crate::api_key_policy::unix_month_start(created_at));
    let spend_minor_db = quota_monthly_spend_minor
        .filter(|&q| q > 0)
        .map(crate::money::minor_to_db);
    let hash = crate::secrets::api_key_sha256_hex(api_key);
    let preview = crate::secrets::api_key_preview_short(api_key);
    conn.execute(
        "INSERT INTO api_keys (user_id, api_key, api_key_hash, key_preview, created_at, name, description, expires_at,
            quota_monthly_tokens, model_allowlist, ip_allowlist, quota_period_start, team_id, default_byok_profile_id,
            max_concurrent_requests, quota_monthly_spend_minor, quota_used_spend_minor, quota_spend_period_start)
         VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, '0', ?16)",
        params![
            user_id,
            hash,
            preview,
            created_at,
            name,
            description,
            expires_at,
            quota_monthly_tokens,
            model_allowlist,
            ip_allowlist,
            period,
            team_id,
            default_byok_profile_id,
            max_concurrent_requests,
            spend_minor_db,
            spend_period,
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
    pub default_byok_profile_id: Option<Option<i64>>,
    pub max_concurrent_requests: Option<Option<i32>>,
    pub quota_monthly_spend_minor: Option<Option<i128>>,
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
    if let Some(ref d) = patch.default_byok_profile_id {
        total += match d {
            Some(pid) => conn.execute(
                "UPDATE api_keys SET default_byok_profile_id = ?1 WHERE id = ?2 AND user_id = ?3 AND revoked = 0",
                params![*pid, key_id, user_id],
            )?,
            None => conn.execute(
                "UPDATE api_keys SET default_byok_profile_id = NULL WHERE id = ?1 AND user_id = ?2 AND revoked = 0",
                params![key_id, user_id],
            )?,
        };
    }
    if let Some(m) = &patch.max_concurrent_requests {
        total += match m {
            None => conn.execute(
                "UPDATE api_keys SET max_concurrent_requests = NULL WHERE id = ?1 AND user_id = ?2 AND revoked = 0",
                params![key_id, user_id],
            )?,
            Some(v) if *v > 0 => conn.execute(
                "UPDATE api_keys SET max_concurrent_requests = ?1 WHERE id = ?2 AND user_id = ?3 AND revoked = 0",
                params![*v, key_id, user_id],
            )?,
            Some(_) => conn.execute(
                "UPDATE api_keys SET max_concurrent_requests = NULL WHERE id = ?1 AND user_id = ?2 AND revoked = 0",
                params![key_id, user_id],
            )?,
        };
    }
    if let Some(s) = &patch.quota_monthly_spend_minor {
        total += match s {
            None => conn.execute(
                "UPDATE api_keys SET quota_monthly_spend_minor = NULL, quota_used_spend_minor = '0', quota_spend_period_start = NULL WHERE id = ?1 AND user_id = ?2 AND revoked = 0",
                params![key_id, user_id],
            )?,
            Some(v) if *v > 0 => {
                let month_start = crate::api_key_policy::unix_month_start(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64,
                );
                conn.execute(
                    "UPDATE api_keys SET quota_monthly_spend_minor = ?1, quota_used_spend_minor = '0', quota_spend_period_start = ?2 WHERE id = ?3 AND user_id = ?4 AND revoked = 0",
                    params![crate::money::minor_to_db(*v), month_start, key_id, user_id],
                )?
            }
            Some(_) => conn.execute(
                "UPDATE api_keys SET quota_monthly_spend_minor = NULL, quota_used_spend_minor = '0', quota_spend_period_start = NULL WHERE id = ?1 AND user_id = ?2 AND revoked = 0",
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
        "UPDATE api_keys SET revoked = 1 WHERE id = ?1 AND user_id = ?2 AND team_id IS NULL AND revoked = 0",
        params![key_id, user_id],
    )?;
    Ok(n)
}

/// Creator may revoke their own team-scoped key; personal keys use [revoke_api_key_for_user].
pub fn revoke_team_api_key_for_creator(
    conn: &Connection,
    creator_user_id: i64,
    key_id: i64,
) -> rusqlite::Result<usize> {
    let n = conn.execute(
        "UPDATE api_keys SET revoked = 1 WHERE id = ?1 AND user_id = ?2 AND team_id IS NOT NULL AND revoked = 0",
        params![key_id, creator_user_id],
    )?;
    Ok(n)
}

/// Fetch key if visible in console: personal owner or any team member (team keys).
pub fn get_api_key_row_for_console(
    conn: &Connection,
    viewer_user_id: i64,
    key_id: i64,
) -> rusqlite::Result<ApiKeyRow> {
    let row = get_api_key_row_by_id(conn, key_id)?;
    if row.team_id.is_none() {
        if row.revoked != 0 || row.user_id != viewer_user_id {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        return Ok(row);
    }
    let tid = row.team_id.unwrap();
    if !user_is_team_member(conn, tid, viewer_user_id)? {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(row)
}

/// Personal key: must be owner. Team key: must be creating user (same as revoke policy for v1).
pub fn update_api_key_for_console(
    conn: &Connection,
    viewer_user_id: i64,
    key_id: i64,
    patch: &ApiKeyPatchDb,
) -> rusqlite::Result<usize> {
    let row = get_api_key_row_by_id(conn, key_id)?;
    if row.user_id != viewer_user_id {
        return Ok(0);
    }
    if row.team_id.is_none() {
        return update_api_key_for_user(conn, viewer_user_id, key_id, patch);
    }
    let tid = row.team_id.unwrap();
    if !user_is_team_member(conn, tid, viewer_user_id)? {
        return Ok(0);
    }
    update_api_key_by_id(conn, key_id, patch)
}

/// Revoke if viewer is the key creator (team or personal).
pub fn revoke_api_key_for_console(
    conn: &Connection,
    viewer_user_id: i64,
    key_id: i64,
) -> rusqlite::Result<usize> {
    let row = get_api_key_row_by_id(conn, key_id)?;
    if row.user_id != viewer_user_id {
        return Ok(0);
    }
    if row.team_id.is_none() {
        return revoke_api_key_for_user(conn, viewer_user_id, key_id);
    }
    let tid = row.team_id.unwrap();
    if !user_is_team_member(conn, tid, viewer_user_id)? {
        return Ok(0);
    }
    revoke_api_key_by_id(conn, key_id)
}

fn update_api_key_by_id(
    conn: &Connection,
    key_id: i64,
    patch: &ApiKeyPatchDb,
) -> rusqlite::Result<usize> {
    let mut total: usize = 0;
    if let Some(ref n) = patch.name {
        total += conn.execute(
            "UPDATE api_keys SET name = ?1 WHERE id = ?2 AND revoked = 0",
            params![n, key_id],
        )?;
    }
    if let Some(ref d) = patch.description {
        total += conn.execute(
            "UPDATE api_keys SET description = ?1 WHERE id = ?2 AND revoked = 0",
            params![d, key_id],
        )?;
    }
    if let Some(d) = patch.disabled {
        total += conn.execute(
            "UPDATE api_keys SET disabled = ?1 WHERE id = ?2 AND revoked = 0",
            params![if d { 1 } else { 0 }, key_id],
        )?;
    }
    if let Some(ref e) = patch.expires_at {
        total += match e {
            Some(ts) => conn.execute(
                "UPDATE api_keys SET expires_at = ?1 WHERE id = ?2 AND revoked = 0",
                params![*ts, key_id],
            )?,
            None => conn.execute(
                "UPDATE api_keys SET expires_at = NULL WHERE id = ?1 AND revoked = 0",
                params![key_id],
            )?,
        };
    }
    if let Some(ref q) = patch.quota_monthly_tokens {
        total += match q {
            Some(v) => conn.execute(
                "UPDATE api_keys SET quota_monthly_tokens = ?1 WHERE id = ?2 AND revoked = 0",
                params![*v, key_id],
            )?,
            None => conn.execute(
                "UPDATE api_keys SET quota_monthly_tokens = NULL, quota_used_tokens = 0, quota_period_start = NULL WHERE id = ?1 AND revoked = 0",
                params![key_id],
            )?,
        };
    }
    if let Some(ref m) = patch.model_allowlist {
        total += match m {
            Some(s) => conn.execute(
                "UPDATE api_keys SET model_allowlist = ?1 WHERE id = ?2 AND revoked = 0",
                params![s, key_id],
            )?,
            None => conn.execute(
                "UPDATE api_keys SET model_allowlist = NULL WHERE id = ?1 AND revoked = 0",
                params![key_id],
            )?,
        };
    }
    if let Some(ref ip) = patch.ip_allowlist {
        total += match ip {
            Some(s) => conn.execute(
                "UPDATE api_keys SET ip_allowlist = ?1 WHERE id = ?2 AND revoked = 0",
                params![s, key_id],
            )?,
            None => conn.execute(
                "UPDATE api_keys SET ip_allowlist = NULL WHERE id = ?1 AND revoked = 0",
                params![key_id],
            )?,
        };
    }
    if let Some(ref d) = patch.default_byok_profile_id {
        total += match d {
            Some(pid) => conn.execute(
                "UPDATE api_keys SET default_byok_profile_id = ?1 WHERE id = ?2 AND revoked = 0",
                params![*pid, key_id],
            )?,
            None => conn.execute(
                "UPDATE api_keys SET default_byok_profile_id = NULL WHERE id = ?1 AND revoked = 0",
                params![key_id],
            )?,
        };
    }
    if let Some(m) = &patch.max_concurrent_requests {
        total += match m {
            None => conn.execute(
                "UPDATE api_keys SET max_concurrent_requests = NULL WHERE id = ?1 AND revoked = 0",
                params![key_id],
            )?,
            Some(v) if *v > 0 => conn.execute(
                "UPDATE api_keys SET max_concurrent_requests = ?1 WHERE id = ?2 AND revoked = 0",
                params![*v, key_id],
            )?,
            Some(_) => conn.execute(
                "UPDATE api_keys SET max_concurrent_requests = NULL WHERE id = ?1 AND revoked = 0",
                params![key_id],
            )?,
        };
    }
    if let Some(s) = &patch.quota_monthly_spend_minor {
        total += match s {
            None => conn.execute(
                "UPDATE api_keys SET quota_monthly_spend_minor = NULL, quota_used_spend_minor = '0', quota_spend_period_start = NULL WHERE id = ?1 AND revoked = 0",
                params![key_id],
            )?,
            Some(v) if *v > 0 => {
                let month_start = crate::api_key_policy::unix_month_start(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64,
                );
                conn.execute(
                    "UPDATE api_keys SET quota_monthly_spend_minor = ?1, quota_used_spend_minor = '0', quota_spend_period_start = ?2 WHERE id = ?3 AND revoked = 0",
                    params![crate::money::minor_to_db(*v), month_start, key_id],
                )?
            }
            Some(_) => conn.execute(
                "UPDATE api_keys SET quota_monthly_spend_minor = NULL, quota_used_spend_minor = '0', quota_spend_period_start = NULL WHERE id = ?1 AND revoked = 0",
                params![key_id],
            )?,
        };
    }
    Ok(total)
}

fn revoke_api_key_by_id(conn: &Connection, key_id: i64) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE api_keys SET revoked = 1 WHERE id = ?1 AND revoked = 0",
        params![key_id],
    )
}

pub fn find_user_id(conn: &Connection, username: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT id FROM users WHERE username = ?1",
        params![username],
        |row| row.get(0),
    )
}

// --- Teams -----------------------------------------------------------------

pub fn user_is_team_member(
    conn: &Connection,
    team_id: i64,
    user_id: i64,
) -> rusqlite::Result<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(1) FROM team_members WHERE team_id = ?1 AND user_id = ?2",
        params![team_id, user_id],
        |row| row.get(0),
    )?;
    Ok(n > 0)
}

pub fn team_member_role(
    conn: &Connection,
    team_id: i64,
    user_id: i64,
) -> rusqlite::Result<Option<String>> {
    let mut stmt =
        conn.prepare("SELECT role FROM team_members WHERE team_id = ?1 AND user_id = ?2")?;
    let mut rows = stmt.query_map(params![team_id, user_id], |row| row.get::<_, String>(0))?;
    match rows.next() {
        Some(Ok(r)) => Ok(Some(r)),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

pub fn is_team_admin_or_owner(
    conn: &Connection,
    team_id: i64,
    user_id: i64,
) -> rusqlite::Result<bool> {
    Ok(matches!(
        team_member_role(conn, team_id, user_id)?.as_deref(),
        Some("owner") | Some("admin")
    ))
}

#[derive(Debug, Clone)]
pub struct TeamRow {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub created_by_user_id: i64,
    pub created_at: i64,
}

pub fn insert_team(
    conn: &Connection,
    name: &str,
    slug: &str,
    created_by_user_id: i64,
    created_at: i64,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO teams (name, slug, created_by_user_id, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![name, slug, created_by_user_id, created_at],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn add_team_member(
    conn: &Connection,
    team_id: i64,
    user_id: i64,
    role: &str,
    joined_at: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO team_members (team_id, user_id, role, joined_at) VALUES (?1, ?2, ?3, ?4)",
        params![team_id, user_id, role, joined_at],
    )?;
    Ok(())
}

pub fn list_teams_for_user(conn: &Connection, user_id: i64) -> rusqlite::Result<Vec<TeamRow>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.slug, t.created_by_user_id, t.created_at
         FROM teams t
         INNER JOIN team_members m ON m.team_id = t.id AND m.user_id = ?1
         ORDER BY t.name COLLATE NOCASE ASC",
    )?;
    let rows = stmt.query_map(params![user_id], |row| {
        Ok(TeamRow {
            id: row.get(0)?,
            name: row.get(1)?,
            slug: row.get(2)?,
            created_by_user_id: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    rows.collect()
}

pub fn get_team_by_id(conn: &Connection, team_id: i64) -> rusqlite::Result<TeamRow> {
    conn.query_row(
        "SELECT id, name, slug, created_by_user_id, created_at FROM teams WHERE id = ?1",
        params![team_id],
        |row| {
            Ok(TeamRow {
                id: row.get(0)?,
                name: row.get(1)?,
                slug: row.get(2)?,
                created_by_user_id: row.get(3)?,
                created_at: row.get(4)?,
            })
        },
    )
}

pub fn update_team_name_slug(
    conn: &Connection,
    team_id: i64,
    name: Option<&str>,
    slug: Option<&str>,
) -> rusqlite::Result<()> {
    if let Some(n) = name {
        conn.execute(
            "UPDATE teams SET name = ?1 WHERE id = ?2",
            params![n, team_id],
        )?;
    }
    if let Some(s) = slug {
        conn.execute(
            "UPDATE teams SET slug = ?1 WHERE id = ?2",
            params![s, team_id],
        )?;
    }
    Ok(())
}

pub fn delete_team(conn: &Connection, team_id: i64) -> rusqlite::Result<usize> {
    conn.execute("DELETE FROM teams WHERE id = ?1", params![team_id])
}

pub fn count_team_owners(conn: &Connection, team_id: i64) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(1) FROM team_members WHERE team_id = ?1 AND role = 'owner'",
        params![team_id],
        |row| row.get(0),
    )
}

#[derive(Debug, Clone)]
pub struct TeamMemberRow {
    pub user_id: i64,
    pub username: String,
    pub role: String,
    pub joined_at: i64,
}

pub fn list_team_members(conn: &Connection, team_id: i64) -> rusqlite::Result<Vec<TeamMemberRow>> {
    let mut stmt = conn.prepare(
        "SELECT m.user_id, u.username, m.role, m.joined_at
         FROM team_members m
         INNER JOIN users u ON u.id = m.user_id
         WHERE m.team_id = ?1
         ORDER BY m.role = 'owner' DESC, u.username COLLATE NOCASE ASC",
    )?;
    let rows = stmt.query_map(params![team_id], |row| {
        Ok(TeamMemberRow {
            user_id: row.get(0)?,
            username: row.get(1)?,
            role: row.get(2)?,
            joined_at: row.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn remove_team_member(
    conn: &Connection,
    team_id: i64,
    user_id: i64,
) -> rusqlite::Result<usize> {
    conn.execute(
        "DELETE FROM team_members WHERE team_id = ?1 AND user_id = ?2",
        params![team_id, user_id],
    )
}

pub fn set_team_member_role(
    conn: &Connection,
    team_id: i64,
    user_id: i64,
    role: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE team_members SET role = ?1 WHERE team_id = ?2 AND user_id = ?3",
        params![role, team_id, user_id],
    )
}

#[derive(Debug, Clone)]
pub struct TeamInvitationRow {
    pub id: i64,
    pub team_id: i64,
    pub invitee_username: String,
    pub role: String,
    pub token_hash: String,
    pub created_by_user_id: i64,
    pub created_at: i64,
    pub expires_at: i64,
    pub accepted_at: Option<i64>,
}

#[allow(clippy::too_many_arguments)]
pub fn insert_team_invitation(
    conn: &Connection,
    team_id: i64,
    invitee_username: &str,
    role: &str,
    token_hash: &str,
    created_by_user_id: i64,
    created_at: i64,
    expires_at: i64,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO team_invitations (team_id, invitee_username, role, token_hash, created_by_user_id, created_at, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            team_id,
            invitee_username,
            role,
            token_hash,
            created_by_user_id,
            created_at,
            expires_at
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn find_pending_invitation_by_hash(
    conn: &Connection,
    token_hash: &str,
) -> rusqlite::Result<TeamInvitationRow> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    conn.query_row(
        "SELECT id, team_id, invitee_username, role, token_hash, created_by_user_id, created_at, expires_at, accepted_at
         FROM team_invitations WHERE token_hash = ?1 AND accepted_at IS NULL AND expires_at > ?2",
        params![token_hash, now],
        |row| {
            Ok(TeamInvitationRow {
                id: row.get(0)?,
                team_id: row.get(1)?,
                invitee_username: row.get(2)?,
                role: row.get(3)?,
                token_hash: row.get(4)?,
                created_by_user_id: row.get(5)?,
                created_at: row.get(6)?,
                expires_at: row.get(7)?,
                accepted_at: row.get(8)?,
            })
        },
    )
}

pub fn mark_invitation_accepted(
    conn: &Connection,
    invitation_id: i64,
    at: i64,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE team_invitations SET accepted_at = ?1 WHERE id = ?2 AND accepted_at IS NULL",
        params![at, invitation_id],
    )
}

pub fn delete_team_invitation(
    conn: &Connection,
    invitation_id: i64,
    team_id: i64,
) -> rusqlite::Result<usize> {
    conn.execute(
        "DELETE FROM team_invitations WHERE id = ?1 AND team_id = ?2",
        params![invitation_id, team_id],
    )
}

pub fn list_pending_invitations_for_team(
    conn: &Connection,
    team_id: i64,
) -> rusqlite::Result<Vec<TeamInvitationRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, team_id, invitee_username, role, token_hash, created_by_user_id, created_at, expires_at, accepted_at
         FROM team_invitations WHERE team_id = ?1 AND accepted_at IS NULL ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(params![team_id], |row| {
        Ok(TeamInvitationRow {
            id: row.get(0)?,
            team_id: row.get(1)?,
            invitee_username: row.get(2)?,
            role: row.get(3)?,
            token_hash: row.get(4)?,
            created_by_user_id: row.get(5)?,
            created_at: row.get(6)?,
            expires_at: row.get(7)?,
            accepted_at: row.get(8)?,
        })
    })?;
    rows.collect()
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

/// Returns `None` if the user row is missing; otherwise the nullable `password_hash` column.
pub fn get_user_password_hash_by_id(
    conn: &Connection,
    user_id: i64,
) -> rusqlite::Result<Option<Option<String>>> {
    conn.query_row(
        "SELECT password_hash FROM users WHERE id = ?1",
        params![user_id],
        |row| row.get::<_, Option<String>>(0),
    )
    .optional()
}

pub fn set_user_password_hash(
    conn: &Connection,
    user_id: i64,
    password_hash: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE users SET password_hash = ?1 WHERE id = ?2",
        params![password_hash, user_id],
    )
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
        "SELECT api_key FROM api_keys WHERE user_id = ?1 AND revoked = 0 AND api_key IS NOT NULL AND LENGTH(TRIM(api_key)) > 0 ORDER BY id DESC LIMIT 1",
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
    let hash = crate::secrets::api_key_sha256_hex(api_key);
    conn.query_row(
        "SELECT 1 FROM api_keys WHERE (api_key_hash = ?1 OR api_key = ?2) AND revoked = 0 AND disabled = 0
         AND (expires_at IS NULL OR expires_at > ?3)",
        params![hash, api_key, now],
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
    let hash = crate::secrets::api_key_sha256_hex(api_key);
    conn.query_row(
        "SELECT id, user_id FROM api_keys WHERE (api_key_hash = ?1 OR api_key = ?2) AND revoked = 0 AND disabled = 0
         AND (expires_at IS NULL OR expires_at > ?3)",
        params![hash, api_key, now],
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
    pub team_id: Option<i64>,
    pub default_byok_profile_id: Option<i64>,
    pub max_concurrent_requests: Option<i32>,
}

pub fn get_api_key_auth_row(conn: &Connection, api_key: &str) -> rusqlite::Result<ApiKeyAuthRow> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let hash = crate::secrets::api_key_sha256_hex(api_key);
    conn.query_row(
        "SELECT id, user_id, model_allowlist, ip_allowlist, quota_monthly_tokens,
                quota_used_tokens, quota_period_start, team_id, default_byok_profile_id, max_concurrent_requests
         FROM api_keys WHERE (api_key_hash = ?1 OR api_key = ?2) AND revoked = 0 AND disabled = 0
         AND (expires_at IS NULL OR expires_at > ?3)",
        params![hash, api_key, now],
        |row| {
            Ok(ApiKeyAuthRow {
                id: row.get(0)?,
                user_id: row.get(1)?,
                model_allowlist: row.get(2)?,
                ip_allowlist: row.get(3)?,
                quota_monthly_tokens: row.get(4)?,
                quota_used_tokens: row.get(5)?,
                quota_period_start: row.get(6)?,
                team_id: row.get(7)?,
                default_byok_profile_id: row.get(8)?,
                max_concurrent_requests: row.get(9)?,
            })
        },
    )
}

/// Reset monthly quota if we crossed into a new UTC calendar month; then check headroom.
pub fn ensure_monthly_quota(conn: &Connection, key_id: i64, now: i64) -> Result<(), &'static str> {
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

/// Reset monthly spend counter if we crossed into a new UTC calendar month; then check headroom.
pub fn ensure_monthly_spend_quota(
    conn: &Connection,
    key_id: i64,
    now: i64,
) -> Result<(), &'static str> {
    use crate::api_key_policy::unix_month_start;
    let month_start = unix_month_start(now);
    let (limit_s, used_s, period): (Option<String>, String, Option<i64>) = conn
        .query_row(
            "SELECT quota_monthly_spend_minor, quota_used_spend_minor, quota_spend_period_start FROM api_keys WHERE id = ?1",
            params![key_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| "api key not found")?;
    let Some(limit_s) = limit_s else {
        return Ok(());
    };
    let limit = match crate::money::minor_from_db(limit_s.trim()) {
        Ok(v) if v > 0 => v,
        _ => return Ok(()),
    };
    let mut used = crate::money::minor_from_db(used_s.trim()).unwrap_or(0);
    if period.map(|p| p < month_start).unwrap_or(true) {
        used = 0;
        conn.execute(
            "UPDATE api_keys SET quota_used_spend_minor = '0', quota_spend_period_start = ?1 WHERE id = ?2",
            params![month_start, key_id],
        )
        .map_err(|_| "database error")?;
    }
    if used >= limit {
        return Err("monthly spend quota exceeded");
    }
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

/// Maintains [`audit_threads`] in the same transaction as audit row inserts.
fn touch_audit_thread(
    conn: &Connection,
    user_id: i64,
    team_id: Option<i64>,
    thread_id: &str,
    at: i64,
) -> rusqlite::Result<()> {
    let thread_id = thread_id.trim();
    if thread_id.is_empty() {
        return Ok(());
    }
    let team_scope = team_id.unwrap_or(0);
    conn.execute(
        "INSERT INTO audit_threads (user_id, team_scope, thread_id, first_seen_at, last_seen_at, request_count)
         VALUES (?1, ?2, ?3, ?4, ?4, 1)
         ON CONFLICT(user_id, team_scope, thread_id) DO UPDATE SET
           last_seen_at = excluded.last_seen_at,
           request_count = audit_threads.request_count + 1",
        params![user_id, team_scope, thread_id, at],
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
                prompt_tokens, completion_tokens, cached_prompt_tokens, total_tokens, cost, latency_ms,
                reasoning_phase_ms,
                app_id, thread_id, finish_reason, metadata, created_at, team_id
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23
            )",
        )?;

        for record in records {
            let metadata = record.metadata.as_ref().map(|v| v.to_string());
            let already_had_row: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM audit_logs WHERE request_id = ?1)",
                params![&record.request_id],
                |row| row.get(0),
            )?;
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
                record.cached_prompt_tokens,
                record.total_tokens,
                record.cost,
                record.latency_ms,
                record.reasoning_phase_ms,
                record.app_id,
                record.thread_id,
                record.finish_reason,
                metadata,
                record.created_at,
                record.team_id,
            ])?;
            if !already_had_row {
                if let (Some(uid), Some(tid)) = (record.user_id, record.thread_id.as_ref()) {
                    touch_audit_thread(&tx, uid, record.team_id, tid, record.created_at)?;
                }
            }
            if let Err(e) = apply_audit_hourly_rollup_from_record(&tx, record) {
                tracing::error!(error = %e, request_id = %record.request_id, "audit_hourly_rollup insert failed");
            }
        }
    }
    tx.commit()
}

pub fn update_audit_log_stream_completion(
    conn: &mut Connection,
    update: &crate::audit::AuditStreamCompletionUpdate,
) -> rusqlite::Result<usize> {
    let metadata = update.metadata.to_string();
    let rows = conn.execute(
        "UPDATE audit_logs SET
            response_body_path = ?1,
            prompt_tokens = ?2,
            completion_tokens = ?3,
            cached_prompt_tokens = ?4,
            total_tokens = ?5,
            cost = ?6,
            finish_reason = ?7,
            latency_ms = ?8,
            reasoning_phase_ms = ?9,
            metadata = ?10,
            error_message = COALESCE(?11, error_message)
         WHERE request_id = ?12",
        params![
            update.response_body_path,
            update.prompt_tokens,
            update.completion_tokens,
            update.cached_prompt_tokens,
            update.total_tokens,
            update.cost,
            update.finish_reason,
            update.latency_ms,
            update.reasoning_phase_ms,
            metadata,
            update.error_message,
            update.request_id,
        ],
    )?;
    if rows > 0 {
        let snapshot = conn.query_row(
            "SELECT user_id, team_id, created_at, status_code, prompt_tokens, completion_tokens, cached_prompt_tokens, total_tokens, cost, latency_ms
             FROM audit_logs WHERE request_id = ?1",
            params![update.request_id],
            |row| {
                Ok((
                    row.get::<_, Option<i64>>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                    row.get::<_, Option<f64>>(8)?,
                    row.get::<_, Option<i64>>(9)?,
                ))
            },
        );
        if let Ok((uid, tid, created_at, sc, pt, ct, cpt, tt, cost, lat)) = snapshot {
            if let Err(e) = apply_audit_hourly_rollup_from_audit_row(
                conn, uid, tid, created_at, sc, pt, ct, cpt, tt, cost, lat,
            ) {
                tracing::error!(
                    error = %e,
                    request_id = %update.request_id,
                    "audit_hourly_rollup stream completion failed"
                );
            }
        }
    }
    Ok(rows)
}

pub fn update_audit_log_request_body_path(
    conn: &mut Connection,
    request_id: &str,
    path: &str,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE audit_logs SET request_body_path = ?1 WHERE request_id = ?2",
        params![path, request_id],
    )
}

/// Marks an accept-phase audit row as failed (quota, IP, BYOK resolve, …) and applies hourly rollup.
pub fn update_audit_log_rejected(
    conn: &mut Connection,
    request_id: &str,
    status_code: i64,
    error_message: &str,
    latency_ms: i64,
) -> rusqlite::Result<()> {
    let rows = conn.execute(
        "UPDATE audit_logs SET status_code = ?1, error_message = ?2, latency_ms = ?3 WHERE request_id = ?4",
        params![status_code, error_message, latency_ms, request_id],
    )?;
    if rows == 0 {
        return Ok(());
    }
    let snapshot = conn.query_row(
        "SELECT user_id, team_id, created_at, status_code, prompt_tokens, completion_tokens, cached_prompt_tokens, total_tokens, cost, latency_ms
         FROM audit_logs WHERE request_id = ?1",
        params![request_id],
        |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, Option<i64>>(7)?,
                row.get::<_, Option<f64>>(8)?,
                row.get::<_, Option<i64>>(9)?,
            ))
        },
    );
    if let Ok((uid, tid, created_at, sc, pt, ct, cpt, tt, cost, lat)) = snapshot {
        if let Err(e) = apply_audit_hourly_rollup_from_audit_row(
            conn, uid, tid, created_at, sc, pt, ct, cpt, tt, cost, lat,
        ) {
            tracing::error!(
                error = %e,
                %request_id,
                "audit_hourly_rollup rejected update failed"
            );
        }
    }
    Ok(())
}

pub fn query_audit_logs(
    conn: &Connection,
    query: &AuditListQuery,
    scope: AuditConsoleScope,
) -> rusqlite::Result<(Vec<AuditListItem>, i64)> {
    let (where_sql, where_args) = build_audit_where_clause(query, scope);
    let limit = query.limit.unwrap_or(100).clamp(1, 1000);
    let offset = query.offset.unwrap_or(0);

    let list_sql = format!(
        "SELECT
            request_id, user_id, team_id, token_id, channel_id, model, request_type,
            status_code, error_message, prompt_tokens, completion_tokens, cached_prompt_tokens,
            total_tokens, cost, latency_ms, reasoning_phase_ms, app_id, thread_id, finish_reason, created_at
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
            team_id: row.get(2)?,
            token_id: row.get(3)?,
            channel_id: row.get(4)?,
            model: row.get(5)?,
            request_type: row.get(6)?,
            status_code: row.get(7)?,
            error_message: row.get(8)?,
            prompt_tokens: row.get(9)?,
            completion_tokens: row.get(10)?,
            cached_prompt_tokens: row.get(11)?,
            total_tokens: row.get(12)?,
            cost: row.get(13)?,
            latency_ms: row.get(14)?,
            reasoning_phase_ms: row.get(15)?,
            app_id: row.get(16)?,
            thread_id: row.get(17)?,
            finish_reason: row.get(18)?,
            created_at: row.get(19)?,
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

pub fn audit_record_visible_to_user(
    conn: &Connection,
    record: &AuditRecord,
    viewer_user_id: i64,
) -> rusqlite::Result<bool> {
    match record.team_id {
        None => Ok(record.user_id == Some(viewer_user_id)),
        Some(tid) => user_is_team_member(conn, tid, viewer_user_id),
    }
}

pub fn get_audit_log_by_request_id(
    conn: &Connection,
    request_id: &str,
    viewer_user_id: i64,
) -> rusqlite::Result<AuditRecord> {
    let sql = "SELECT
            request_id, user_id, token_id, channel_id, model, request_type,
            request_body_path, response_body_path, status_code, error_message,
            prompt_tokens, completion_tokens, cached_prompt_tokens, total_tokens, cost, latency_ms,
            reasoning_phase_ms,
            app_id, thread_id, finish_reason, metadata, created_at, team_id
         FROM audit_logs
         WHERE request_id = ?1";

    let record = conn.query_row(sql, params![request_id], |row| {
        let metadata_str: Option<String> = row.get(20)?;
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
            cached_prompt_tokens: row.get(12)?,
            total_tokens: row.get(13)?,
            cost: row.get(14)?,
            latency_ms: row.get(15)?,
            reasoning_phase_ms: row.get(16)?,
            app_id: row.get(17)?,
            thread_id: row.get(18)?,
            finish_reason: row.get(19)?,
            metadata,
            created_at: row.get(21)?,
            team_id: row.get(22)?,
        })
    })?;

    if !audit_record_visible_to_user(conn, &record, viewer_user_id)? {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(record)
}

fn audit_column(alias: &str, name: &str) -> String {
    if alias.is_empty() {
        name.to_string()
    } else {
        format!("{alias}.{name}")
    }
}

fn audit_metadata_accept_expr(alias: &str) -> String {
    if alias.is_empty() {
        "json_extract(metadata, '$.accept_phase')".to_string()
    } else {
        format!("json_extract({alias}.metadata, '$.accept_phase')")
    }
}

/// When `include_scope` is false, caller must constrain rows elsewhere (e.g. `EXISTS` correlation).
pub(crate) fn audit_log_where_parts(
    query: &AuditListQuery,
    scope: AuditConsoleScope,
    alias: &str,
    include_scope: bool,
) -> (Vec<String>, Vec<Value>) {
    let mut where_clauses: Vec<String> = Vec::new();
    let mut args: Vec<Value> = Vec::new();
    let c = |name: &str| audit_column(alias, name);

    if include_scope {
        match scope {
            AuditConsoleScope::Personal(user_id) => {
                where_clauses.push(format!("{} = ?", c("user_id")));
                args.push(Value::Integer(user_id));
                where_clauses.push(format!("{} IS NULL", c("team_id")));
            }
            AuditConsoleScope::Team(team_id) => {
                where_clauses.push(format!("{} = ?", c("team_id")));
                args.push(Value::Integer(team_id));
            }
            AuditConsoleScope::UserOwnedTraffic(user_id) => {
                where_clauses.push(format!("{} = ?", c("user_id")));
                args.push(Value::Integer(user_id));
            }
        }
    }
    if let Some(token_id) = query.token_id {
        where_clauses.push(format!("{} = ?", c("token_id")));
        args.push(Value::Integer(token_id));
    }
    if let Some(start_time) = query.start_time {
        where_clauses.push(format!("{} >= ?", c("created_at")));
        args.push(Value::Integer(start_time));
    }
    if let Some(end_time) = query.end_time {
        where_clauses.push(format!("{} <= ?", c("created_at")));
        args.push(Value::Integer(end_time));
    }
    if let Some(status_code) = query.status_code {
        where_clauses.push(format!("{} = ?", c("status_code")));
        args.push(Value::Integer(status_code));
    }
    if let Some(channel_id) = &query.channel_id {
        where_clauses.push(format!("{} = ?", c("channel_id")));
        args.push(Value::Text(channel_id.clone()));
    }
    if let Some(model) = &query.model {
        where_clauses.push(format!("{} = ?", c("model")));
        args.push(Value::Text(model.clone()));
    }
    if let Some(keyword) = &query.keyword {
        let like_kw = format!("%{keyword}%");
        where_clauses.push(format!(
            "({} LIKE ? OR {} LIKE ? OR {} LIKE ?)",
            c("request_id"),
            c("error_message"),
            c("model")
        ));
        args.push(Value::Text(like_kw.clone()));
        args.push(Value::Text(like_kw.clone()));
        args.push(Value::Text(like_kw));
    }
    if let Some(app_id) = &query.app_id {
        if !app_id.is_empty() {
            where_clauses.push(format!("{} = ?", c("app_id")));
            args.push(Value::Text(app_id.clone()));
        }
    }
    if let Some(thread_id) = &query.thread_id {
        if !thread_id.is_empty() {
            where_clauses.push(format!("{} = ?", c("thread_id")));
            args.push(Value::Text(thread_id.clone()));
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
            where_clauses.push(format!(
                "{} IN ({})",
                c("finish_reason"),
                placeholders.join(", ")
            ));
            for p in parts {
                args.push(Value::Text(p));
            }
        }
    }
    if let Some(v) = query.min_prompt_tokens {
        where_clauses.push(format!("{} >= ?", c("prompt_tokens")));
        args.push(Value::Integer(v));
    }
    if let Some(v) = query.max_prompt_tokens {
        where_clauses.push(format!("{} <= ?", c("prompt_tokens")));
        args.push(Value::Integer(v));
    }
    if let Some(v) = query.min_completion_tokens {
        where_clauses.push(format!("{} >= ?", c("completion_tokens")));
        args.push(Value::Integer(v));
    }
    if let Some(v) = query.max_completion_tokens {
        where_clauses.push(format!("{} <= ?", c("completion_tokens")));
        args.push(Value::Integer(v));
    }

    let meta_expr = audit_metadata_accept_expr(alias);
    where_clauses.push(format!(
        "NOT ({} IS NULL AND COALESCE({}, 0) = 1)",
        c("status_code"),
        meta_expr
    ));

    (where_clauses, args)
}

fn build_audit_where_clause(
    query: &AuditListQuery,
    scope: AuditConsoleScope,
) -> (String, Vec<Value>) {
    let (parts, args) = audit_log_where_parts(query, scope, "", true);
    let where_sql = if parts.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", parts.join(" AND "))
    };
    (where_sql, args)
}

/// Session/thread summary rows: one row per non-empty `thread_id` group from `audit_logs` under the same filters as [`query_audit_logs`].
pub fn query_audit_threads(
    conn: &Connection,
    query: &AuditListQuery,
    scope: AuditConsoleScope,
) -> rusqlite::Result<(Vec<AuditThreadListItem>, i64)> {
    let (mut inner_parts, inner_args) = audit_log_where_parts(query, scope, "al", true);
    debug_assert!(
        !inner_parts.is_empty(),
        "audit log filter parts should always include accept_phase guard"
    );
    inner_parts.push("al.thread_id IS NOT NULL".to_string());
    inner_parts.push("length(trim(al.thread_id)) > 0".to_string());
    let where_sql = format!("WHERE {}", inner_parts.join(" AND "));

    let limit = query.limit.unwrap_or(100).clamp(1, 1000);
    let offset = query.offset.unwrap_or(0);

    let count_sql = format!(
        "SELECT COUNT(*) FROM (\
            SELECT 1 FROM audit_logs al {where_sql} \
            GROUP BY al.user_id, COALESCE(al.team_id, 0), trim(al.thread_id)\
        )"
    );
    let total: i64 = conn.query_row(&count_sql, params_from_iter(inner_args.iter()), |row| {
        row.get(0)
    })?;

    let list_sql = format!(
        "SELECT \
            trim(al.thread_id) AS thread_id, \
            al.user_id, \
            COALESCE(al.team_id, 0) AS team_scope, \
            MIN(al.created_at) AS first_seen_at, \
            MAX(al.created_at) AS last_seen_at, \
            COUNT(*) AS request_count, \
            COALESCE(SUM(al.prompt_tokens), 0) AS total_prompt_tokens, \
            COALESCE(SUM(al.completion_tokens), 0) AS total_completion_tokens, \
            COALESCE(SUM(al.total_tokens), 0) AS total_tokens, \
            COALESCE(SUM(al.cached_prompt_tokens), 0) AS total_cached_prompt_tokens, \
            COALESCE(SUM(al.cost), 0.0) AS total_cost, \
            SUM(CASE WHEN al.status_code IS NOT NULL AND al.status_code >= 400 THEN 1 ELSE 0 END) AS error_count \
         FROM audit_logs al \
         {where_sql} \
         GROUP BY al.user_id, COALESCE(al.team_id, 0), trim(al.thread_id) \
         ORDER BY MAX(al.created_at) DESC \
         LIMIT ? OFFSET ?"
    );

    let mut list_args = inner_args;
    list_args.push(Value::Integer(limit as i64));
    list_args.push(Value::Integer(offset as i64));

    let mut stmt = conn.prepare(&list_sql)?;
    let rows = stmt.query_map(params_from_iter(list_args.iter()), |row| {
        let team_scope: i64 = row.get(2)?;
        Ok(AuditThreadListItem {
            thread_id: row.get(0)?,
            user_id: row.get(1)?,
            team_id: if team_scope == 0 {
                None
            } else {
                Some(team_scope)
            },
            first_seen_at: row.get(3)?,
            last_seen_at: row.get(4)?,
            request_count: row.get(5)?,
            total_prompt_tokens: row.get(6)?,
            total_completion_tokens: row.get(7)?,
            total_tokens: row.get(8)?,
            total_cached_prompt_tokens: row.get(9)?,
            total_cost: row.get(10)?,
            error_count: row.get(11)?,
        })
    })?;

    let mut records = Vec::new();
    for r in rows {
        records.push(r?);
    }

    Ok((records, total))
}

/// True when analytics can use `audit_hourly_rollups` (no extra filters beyond time + scope).
fn audit_analytics_rollup_eligible(query: &AuditListQuery) -> bool {
    query.keyword.is_none()
        && query.model.is_none()
        && query.channel_id.is_none()
        && query.status_code.is_none()
        && query.token_id.is_none()
        && query.app_id.is_none()
        && query.thread_id.is_none()
        && query.finish_reason.is_none()
        && query.min_prompt_tokens.is_none()
        && query.max_prompt_tokens.is_none()
        && query.min_completion_tokens.is_none()
        && query.max_completion_tokens.is_none()
}

/// Inclusive `[eff_start, eff_end]` spans whole hours: `eff_start` aligned to hour and length is a multiple of 3600s.
fn audit_analytics_rollup_aligned_hours(eff_start: i64, eff_end: i64) -> Option<i64> {
    if eff_start % 3600 != 0 {
        return None;
    }
    let span = eff_end.saturating_sub(eff_start).saturating_add(1);
    if span <= 0 || span % 3600 != 0 {
        return None;
    }
    Some(span)
}

fn audit_record_rollup_scope(record: &AuditRecord) -> Option<(&'static str, i64)> {
    match record.team_id {
        Some(tid) => Some(("t", tid)),
        None => record.user_id.map(|u| ("p", u)),
    }
}

fn hour_bucket_start(created_at: i64) -> i64 {
    (created_at / 3600) * 3600
}

/// Skip rollup on initial insert for streaming chat (finalized via `update_audit_log_stream_completion`).
fn audit_record_skip_rollup_on_insert(record: &AuditRecord) -> bool {
    if record
        .metadata
        .as_ref()
        .and_then(|m| m.get("accept_phase"))
        .and_then(|v| v.as_bool())
        == Some(true)
    {
        return true;
    }
    record
        .metadata
        .as_ref()
        .and_then(|m| m.get("stream"))
        .and_then(|v| v.as_bool())
        == Some(true)
}

#[allow(clippy::too_many_arguments)]
fn upsert_audit_hourly_rollup_increment(
    conn: &Connection,
    bucket_start: i64,
    scope: &str,
    scope_id: i64,
    request_count: i64,
    success_count: i64,
    prompt_tokens: i64,
    completion_tokens: i64,
    cached_prompt_tokens: i64,
    total_tokens: i64,
    cost_sum: f64,
    latency_ms: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO audit_hourly_rollups (
            bucket_start, scope, scope_id, request_count, success_count,
            prompt_tokens, completion_tokens, cached_prompt_tokens, total_tokens, cost_sum, latency_ms_sum
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        ON CONFLICT(bucket_start, scope, scope_id) DO UPDATE SET
            request_count = audit_hourly_rollups.request_count + excluded.request_count,
            success_count = audit_hourly_rollups.success_count + excluded.success_count,
            prompt_tokens = audit_hourly_rollups.prompt_tokens + excluded.prompt_tokens,
            completion_tokens = audit_hourly_rollups.completion_tokens + excluded.completion_tokens,
            cached_prompt_tokens = audit_hourly_rollups.cached_prompt_tokens + excluded.cached_prompt_tokens,
            total_tokens = audit_hourly_rollups.total_tokens + excluded.total_tokens,
            cost_sum = audit_hourly_rollups.cost_sum + excluded.cost_sum,
            latency_ms_sum = audit_hourly_rollups.latency_ms_sum + excluded.latency_ms_sum",
        params![
            bucket_start,
            scope,
            scope_id,
            request_count,
            success_count,
            prompt_tokens,
            completion_tokens,
            cached_prompt_tokens,
            total_tokens,
            cost_sum,
            latency_ms,
        ],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_audit_hourly_rollup_from_audit_row(
    conn: &Connection,
    user_id: Option<i64>,
    team_id: Option<i64>,
    created_at: i64,
    status_code: Option<i64>,
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    cached_prompt_tokens: Option<i64>,
    total_tokens: Option<i64>,
    cost: Option<f64>,
    latency_ms: Option<i64>,
) -> rusqlite::Result<()> {
    let (scope, scope_id) = match team_id {
        Some(tid) => ("t", tid),
        None => {
            let Some(uid) = user_id else {
                return Ok(());
            };
            ("p", uid)
        }
    };
    let bucket = hour_bucket_start(created_at);
    let success = if let Some(sc) = status_code {
        i64::from((200..300).contains(&sc))
    } else {
        0
    };
    let pt = prompt_tokens.unwrap_or(0);
    let ct = completion_tokens.unwrap_or(0);
    let cp = cached_prompt_tokens.unwrap_or(0).min(pt);
    let tt = total_tokens.unwrap_or(0);
    let cost_sum = cost.unwrap_or(0.0);
    let lat = latency_ms.unwrap_or(0);
    upsert_audit_hourly_rollup_increment(
        conn, bucket, scope, scope_id, 1, success, pt, ct, cp, tt, cost_sum, lat,
    )
}

fn apply_audit_hourly_rollup_from_record(
    conn: &Connection,
    record: &AuditRecord,
) -> rusqlite::Result<()> {
    if audit_record_skip_rollup_on_insert(record) {
        return Ok(());
    }
    if audit_record_rollup_scope(record).is_none() {
        return Ok(());
    }
    apply_audit_hourly_rollup_from_audit_row(
        conn,
        record.user_id,
        record.team_id,
        record.created_at,
        record.status_code,
        record.prompt_tokens,
        record.completion_tokens,
        record.cached_prompt_tokens,
        record.total_tokens,
        record.cost,
        record.latency_ms,
    )
}

fn audit_analytics_bucket_seconds(range_start: i64, range_end: i64) -> i64 {
    let span = range_end.saturating_sub(range_start).max(1);
    if span <= 2 * 86400 {
        3600
    } else if span <= 90 * 86400 {
        86400
    } else {
        7 * 86400
    }
}

const ANALYTICS_MAX_RANGE_SECS: i64 = 366 * 86400;

fn audit_analytics_series_from_rollup(
    conn: &Connection,
    scope: &str,
    scope_id: i64,
    eff_start: i64,
    span_secs: i64,
) -> rusqlite::Result<Vec<crate::audit::AuditAnalyticsTimeBucket>> {
    use crate::audit::AuditAnalyticsTimeBucket;
    use std::collections::HashMap;

    let end_excl = eff_start.saturating_add(span_secs);
    let mut stmt = conn.prepare(
        "SELECT bucket_start, request_count, total_tokens, cost_sum,
                prompt_tokens, completion_tokens, cached_prompt_tokens
         FROM audit_hourly_rollups
         WHERE scope = ?1 AND scope_id = ?2 AND bucket_start >= ?3 AND bucket_start < ?4
         ORDER BY bucket_start ASC",
    )?;
    let rows = stmt.query_map(params![scope, scope_id, eff_start, end_excl], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, f64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
        ))
    })?;
    let mut by_bucket: HashMap<i64, (i64, i64, f64, i64, i64, i64)> = HashMap::new();
    for r in rows {
        let (bs, rc, tt, cs, pt, ct, cpt) = r?;
        by_bucket.insert(bs, (rc, tt, cs, pt, ct, cpt));
    }
    let mut series = Vec::new();
    let mut cur = eff_start;
    while cur < end_excl {
        let (rc, tt, cs, pt, ct, cpt) =
            by_bucket.get(&cur).copied().unwrap_or((0, 0, 0.0, 0, 0, 0));
        series.push(AuditAnalyticsTimeBucket {
            bucket_start: cur,
            request_count: rc,
            total_tokens: tt,
            total_cost: cs,
            prompt_tokens: pt,
            completion_tokens: ct,
            cached_prompt_tokens: cpt,
        });
        cur = cur.saturating_add(3600);
    }
    Ok(series)
}

pub fn query_audit_analytics(
    conn: &Connection,
    filter: &AuditListQuery,
    scope: AuditConsoleScope,
) -> rusqlite::Result<crate::audit::AuditAnalyticsResponse> {
    use crate::audit::{
        AuditAnalyticsModelSlice, AuditAnalyticsResponse, AuditAnalyticsSummary,
        AuditAnalyticsTimeBucket,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let mut eff_end = filter.end_time.unwrap_or(now);
    let mut eff_start = filter.start_time.unwrap_or(eff_end - 7 * 86400);
    if eff_end < eff_start {
        std::mem::swap(&mut eff_start, &mut eff_end);
    }
    if eff_end - eff_start > ANALYTICS_MAX_RANGE_SECS {
        eff_start = eff_end - ANALYTICS_MAX_RANGE_SECS;
    }

    let mut base = filter.clone();
    base.start_time = Some(eff_start);
    base.end_time = Some(eff_end);
    base.limit = None;
    base.offset = None;

    let bucket_sec = audit_analytics_bucket_seconds(eff_start, eff_end);
    let (where_sql, where_args) = build_audit_where_clause(&base, scope);

    let (rollup_scope, rollup_scope_id) = match scope {
        AuditConsoleScope::Personal(uid) => ("p", uid),
        AuditConsoleScope::Team(tid) => ("t", tid),
        AuditConsoleScope::UserOwnedTraffic(_) => ("", 0),
    };

    let span_hours = audit_analytics_rollup_aligned_hours(eff_start, eff_end);
    let try_rollup = matches!(
        scope,
        AuditConsoleScope::Personal(_) | AuditConsoleScope::Team(_)
    ) && audit_analytics_rollup_eligible(&base)
        && bucket_sec == 3600
        && span_hours.is_some();

    let mut use_rollup = false;
    // When rollup eligibility is checked: (audit_logs row count, rollup request_count sum).
    let mut rollup_vs_audit: Option<(i64, i64)> = None;
    if try_rollup {
        let span = span_hours.unwrap_or(0);
        let count_sql = format!("SELECT COUNT(1) FROM audit_logs {where_sql}");
        let audit_total: i64 =
            conn.query_row(&count_sql, params_from_iter(where_args.iter()), |row| {
                row.get(0)
            })?;
        let rollup_sql = "SELECT COALESCE(SUM(request_count), 0) FROM audit_hourly_rollups
             WHERE scope = ?1 AND scope_id = ?2 AND bucket_start >= ?3 AND bucket_start < ?4";
        let rollup_total: i64 = conn.query_row(
            rollup_sql,
            params![
                rollup_scope,
                rollup_scope_id,
                eff_start,
                eff_start.saturating_add(span)
            ],
            |row| row.get(0),
        )?;
        rollup_vs_audit = Some((audit_total, rollup_total));
        use_rollup = rollup_total > 0 && rollup_total == audit_total;
    }

    let (total_requests, success_requests, total_tokens, total_cost, avg_latency_ms, series) =
        if use_rollup {
            let span = span_hours.unwrap_or(0);
            let end_excl = eff_start.saturating_add(span);
            let summary_rollup = "SELECT COALESCE(SUM(request_count), 0),
                    COALESCE(SUM(success_count), 0),
                    COALESCE(SUM(total_tokens), 0),
                    COALESCE(SUM(cost_sum), 0.0),
                    CASE WHEN SUM(request_count) > 0
                        THEN CAST(SUM(latency_ms_sum) AS REAL) / SUM(request_count)
                        ELSE NULL END
             FROM audit_hourly_rollups
             WHERE scope = ?1 AND scope_id = ?2 AND bucket_start >= ?3 AND bucket_start < ?4";
            let s = conn.query_row(
                summary_rollup,
                params![rollup_scope, rollup_scope_id, eff_start, end_excl],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, f64>(3)?,
                        row.get::<_, Option<f64>>(4)?,
                    ))
                },
            )?;
            let ser = audit_analytics_series_from_rollup(
                conn,
                rollup_scope,
                rollup_scope_id,
                eff_start,
                span,
            )?;
            (s.0, s.1, s.2, s.3, s.4, ser)
        } else {
            let summary_sql = format!(
            "SELECT COUNT(1),
                    COALESCE(SUM(CASE WHEN status_code IS NOT NULL AND status_code >= 200 AND status_code < 300 THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(total_tokens), 0),
                    COALESCE(SUM(cost), 0.0),
                    AVG(CAST(latency_ms AS REAL))
             FROM audit_logs {where_sql}"
        );

            let (total_requests, success_requests, total_tokens, total_cost, avg_latency_ms) = conn
                .query_row(&summary_sql, params_from_iter(where_args.iter()), |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, f64>(3)?,
                        row.get::<_, Option<f64>>(4)?,
                    ))
                })?;

            let series_sql = format!(
                "SELECT (created_at / ?) * ? AS bucket_start,
                    COUNT(1) AS c,
                    COALESCE(SUM(total_tokens), 0) AS t,
                    COALESCE(SUM(cost), 0.0) AS cost_sum,
                    COALESCE(SUM(prompt_tokens), 0) AS pt,
                    COALESCE(SUM(completion_tokens), 0) AS ct,
                    COALESCE(SUM(COALESCE(cached_prompt_tokens, 0)), 0) AS cpt
             FROM audit_logs {where_sql}
             GROUP BY bucket_start
             ORDER BY bucket_start ASC"
            );

            let mut series_bind: Vec<Value> = Vec::new();
            series_bind.push(Value::Integer(bucket_sec));
            series_bind.push(Value::Integer(bucket_sec));
            series_bind.extend(where_args.iter().cloned());

            let mut stmt = conn.prepare(&series_sql)?;
            let series_rows = stmt.query_map(params_from_iter(series_bind.iter()), |row| {
                Ok(AuditAnalyticsTimeBucket {
                    bucket_start: row.get(0)?,
                    request_count: row.get(1)?,
                    total_tokens: row.get(2)?,
                    total_cost: row.get(3)?,
                    prompt_tokens: row.get(4)?,
                    completion_tokens: row.get(5)?,
                    cached_prompt_tokens: row.get(6)?,
                })
            })?;
            let mut series = Vec::new();
            for r in series_rows {
                series.push(r?);
            }
            (
                total_requests,
                success_requests,
                total_tokens,
                total_cost,
                avg_latency_ms,
                series,
            )
        };

    let model_sql = format!(
        "SELECT CASE WHEN model IS NULL OR model = '' THEN '(unknown)' ELSE model END AS m,
                COUNT(1) AS c,
                COALESCE(SUM(total_tokens), 0) AS t
         FROM audit_logs {where_sql}
         GROUP BY CASE WHEN model IS NULL OR model = '' THEN '(unknown)' ELSE model END
         ORDER BY c DESC
         LIMIT 30"
    );
    let mut m_stmt = conn.prepare(&model_sql)?;
    let model_rows = m_stmt.query_map(params_from_iter(where_args.iter()), |row| {
        Ok(AuditAnalyticsModelSlice {
            model: row.get(0)?,
            request_count: row.get(1)?,
            total_tokens: row.get(2)?,
        })
    })?;
    let mut by_model = Vec::new();
    for r in model_rows {
        by_model.push(r?);
    }

    let (series_first_bucket, series_last_bucket) = match (series.first(), series.last()) {
        (Some(a), Some(b)) => (Some(a.bucket_start), Some(b.bucket_start)),
        _ => (None, None),
    };
    let series_buckets_with_usage = series
        .iter()
        .filter(|b| b.request_count > 0 || b.total_cost > 0.0)
        .count();
    debug!(
        target: "audit_analytics",
        ?scope,
        eff_start,
        eff_end,
        range_inclusive_secs = eff_end.saturating_sub(eff_start).saturating_add(1),
        hour_aligned = eff_start % 3600 == 0 && (eff_end.saturating_sub(eff_start).saturating_add(1)) % 3600 == 0,
        bucket_sec,
        try_rollup,
        use_rollup,
        rollup_vs_audit = ?rollup_vs_audit,
        series_len = series.len(),
        series_first_bucket,
        series_last_bucket,
        series_buckets_with_usage,
        total_requests,
        total_cost = total_cost,
        "audit analytics computed"
    );

    Ok(AuditAnalyticsResponse {
        summary: AuditAnalyticsSummary {
            total_requests,
            success_requests,
            total_tokens,
            total_cost,
            avg_latency_ms,
        },
        bucket_seconds: bucket_sec,
        series,
        by_model,
    })
}

// --- Billing: USD as integer minor units, scale k=15 (see `crate::money`) ---

#[derive(Debug)]
pub enum BillingChargeError {
    Insufficient { balance_minor: i128 },
    Database(rusqlite::Error),
}

impl From<rusqlite::Error> for BillingChargeError {
    fn from(e: rusqlite::Error) -> Self {
        BillingChargeError::Database(e)
    }
}

/// Ensure a `user_balances` row exists for `user_id`.
pub fn ensure_user_balance_row(conn: &Connection, user_id: i64) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO user_balances (user_id, balance_minor) VALUES (?1, '0')",
        params![user_id],
    )?;
    Ok(())
}

pub fn get_balance_minor(conn: &Connection, user_id: i64) -> rusqlite::Result<i128> {
    ensure_user_balance_row(conn, user_id)?;
    let s: String = conn.query_row(
        "SELECT balance_minor FROM user_balances WHERE user_id = ?1",
        params![user_id],
        |row| row.get(0),
    )?;
    crate::money::minor_from_db(&s).map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "balance_minor not a valid i128 string",
            )),
        )
    })
}

/// Adds minor units to balance and appends a `deposit` ledger row. `amount_minor` must be positive.
pub fn billing_deposit(
    conn: &Connection,
    user_id: i64,
    amount_minor: i128,
    now: i64,
    external_ref: Option<&str>,
) -> rusqlite::Result<i128> {
    ensure_user_balance_row(conn, user_id)?;
    let cur = get_balance_minor(conn, user_id)?;
    let after = cur.saturating_add(amount_minor);
    conn.execute(
        "UPDATE user_balances SET balance_minor = ?1 WHERE user_id = ?2",
        params![crate::money::minor_to_db(after), user_id],
    )?;
    conn.execute(
        "INSERT INTO billing_ledger (user_id, created_at, kind, amount_minor, balance_after_minor, external_ref)
         VALUES (?1, ?2, 'deposit', ?3, ?4, ?5)",
        params![
            user_id,
            now,
            crate::money::minor_to_db(amount_minor),
            crate::money::minor_to_db(after),
            external_ref
        ],
    )?;
    Ok(after)
}

/// Request metadata for a `usage_charge` ledger row (OpenRouter-style audit fields).
pub struct BillingUsageChargeMeta<'a> {
    pub request_id: &'a str,
    pub model: Option<&'a str>,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
}

fn increment_api_key_spend_in_transaction(
    tx: &rusqlite::Transaction<'_>,
    key_id: i64,
    charge_minor: i128,
) -> rusqlite::Result<()> {
    if charge_minor <= 0 {
        return Ok(());
    }
    let used_s: String = match tx.query_row(
        "SELECT quota_used_spend_minor FROM api_keys WHERE id = ?1 AND quota_monthly_spend_minor IS NOT NULL",
        params![key_id],
        |r| r.get(0),
    ) {
        Ok(u) => u,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(()),
        Err(e) => return Err(e),
    };
    let used = crate::money::minor_from_db(used_s.trim()).unwrap_or(0);
    let new_used = used.saturating_add(charge_minor);
    tx.execute(
        "UPDATE api_keys SET quota_used_spend_minor = ?1 WHERE id = ?2 AND quota_monthly_spend_minor IS NOT NULL",
        params![crate::money::minor_to_db(new_used), key_id],
    )?;
    Ok(())
}

/// Deducts `charge_minor` after a successful upstream chat call. Returns new balance, or error if insufficient funds.
pub fn billing_charge_usage(
    conn: &Connection,
    user_id: i64,
    gateway_key_id: Option<i64>,
    charge_minor: i128,
    now: i64,
    meta: BillingUsageChargeMeta<'_>,
) -> Result<i128, BillingChargeError> {
    if charge_minor <= 0 {
        return Ok(get_balance_minor(conn, user_id)?);
    }
    let tx = conn.unchecked_transaction()?;
    ensure_user_balance_row(&tx, user_id)?;
    let cur = get_balance_minor(&tx, user_id)?;
    if cur < charge_minor {
        return Err(BillingChargeError::Insufficient { balance_minor: cur });
    }
    let new_bal = cur - charge_minor;
    tx.execute(
        "UPDATE user_balances SET balance_minor = ?1 WHERE user_id = ?2",
        params![crate::money::minor_to_db(new_bal), user_id],
    )?;
    let neg = -charge_minor;
    tx.execute(
        "INSERT INTO billing_ledger (user_id, created_at, kind, amount_minor, balance_after_minor, request_id, model, prompt_tokens, completion_tokens)
         VALUES (?1, ?2, 'usage_charge', ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            user_id,
            now,
            crate::money::minor_to_db(neg),
            crate::money::minor_to_db(new_bal),
            meta.request_id,
            meta.model,
            meta.prompt_tokens,
            meta.completion_tokens
        ],
    )?;
    if let Some(kid) = gateway_key_id {
        increment_api_key_spend_in_transaction(&tx, kid, charge_minor)?;
    }
    tx.commit()?;
    Ok(new_bal)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BillingLedgerRow {
    pub id: i64,
    pub created_at: i64,
    pub kind: String,
    pub amount_minor: i128,
    pub balance_after_minor: i128,
    pub request_id: Option<String>,
    pub model: Option<String>,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub external_ref: Option<String>,
}

pub fn list_billing_ledger(
    conn: &Connection,
    user_id: i64,
    kind: Option<&str>,
    limit: i64,
    offset: i64,
) -> rusqlite::Result<Vec<BillingLedgerRow>> {
    let limit = limit.clamp(1, 500);
    let offset = offset.max(0);
    let mut out = Vec::new();
    if let Some(k) = kind {
        let mut stmt = conn.prepare(
            "SELECT id, created_at, kind, amount_minor, balance_after_minor, request_id, model, prompt_tokens, completion_tokens, external_ref
             FROM billing_ledger WHERE user_id = ?1 AND kind = ?2
             ORDER BY created_at DESC LIMIT ?3 OFFSET ?4",
        )?;
        let rows = stmt.query_map(params![user_id, k, limit, offset], map_ledger_row)?;
        for r in rows {
            out.push(r?);
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, created_at, kind, amount_minor, balance_after_minor, request_id, model, prompt_tokens, completion_tokens, external_ref
             FROM billing_ledger WHERE user_id = ?1
             ORDER BY created_at DESC LIMIT ?2 OFFSET ?3",
        )?;
        let rows = stmt.query_map(params![user_id, limit, offset], map_ledger_row)?;
        for r in rows {
            out.push(r?);
        }
    }
    Ok(out)
}

fn map_ledger_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BillingLedgerRow> {
    let am: String = row.get(3)?;
    let ba: String = row.get(4)?;
    Ok(BillingLedgerRow {
        id: row.get(0)?,
        created_at: row.get(1)?,
        kind: row.get(2)?,
        amount_minor: crate::money::minor_from_db(&am).unwrap_or(0),
        balance_after_minor: crate::money::minor_from_db(&ba).unwrap_or(0),
        request_id: row.get(5)?,
        model: row.get(6)?,
        prompt_tokens: row.get(7)?,
        completion_tokens: row.get(8)?,
        external_ref: row.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_key_policy::unix_month_start;
    use chrono::{TimeZone, Utc};
    use rusqlite::Connection;
    use rust_decimal::Decimal;
    use std::str::FromStr;
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

    #[test]
    fn insert_audit_logs_updates_audit_threads() {
        use crate::audit::AuditRecord;

        let mut conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");
        let t0 = 10_000_000_i64;
        let row = |rid: &str, ts: i64| AuditRecord {
            request_id: rid.into(),
            user_id: Some(7),
            token_id: Some(1),
            channel_id: None,
            model: Some("m-a".into()),
            request_type: Some("chat".into()),
            request_body_path: None,
            response_body_path: None,
            status_code: Some(200),
            error_message: None,
            prompt_tokens: Some(1),
            completion_tokens: Some(4),
            cached_prompt_tokens: None,
            total_tokens: Some(5),
            cost: Some(0.02),
            latency_ms: Some(50),
            reasoning_phase_ms: None,
            app_id: None,
            thread_id: Some("thread-a".into()),
            finish_reason: None,
            metadata: None,
            created_at: ts,
            team_id: None,
        };
        insert_audit_logs(&mut conn, &[row("q1", t0), row("q2", t0 + 5)]).expect("insert audit");

        let row: (i64, i64, i64) = conn
            .query_row(
                "SELECT first_seen_at, last_seen_at, request_count FROM audit_threads WHERE user_id = 7 AND team_scope = 0 AND thread_id = 'thread-a'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .expect("audit_threads row");
        assert_eq!(row.0, t0);
        assert_eq!(row.1, t0 + 5);
        assert_eq!(row.2, 2);
    }

    #[test]
    fn query_audit_threads_lists_matching_sessions() {
        use crate::audit::{AuditListQuery, AuditRecord};

        let mut conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");
        let t0 = 10_000_000_i64;
        let row = |rid: &str, ts: i64| AuditRecord {
            request_id: rid.into(),
            user_id: Some(7),
            token_id: Some(1),
            channel_id: None,
            model: Some("m-a".into()),
            request_type: Some("chat".into()),
            request_body_path: None,
            response_body_path: None,
            status_code: Some(200),
            error_message: None,
            prompt_tokens: Some(1),
            completion_tokens: Some(4),
            cached_prompt_tokens: None,
            total_tokens: Some(5),
            cost: Some(0.02),
            latency_ms: Some(50),
            reasoning_phase_ms: None,
            app_id: None,
            thread_id: Some("thread-a".into()),
            finish_reason: None,
            metadata: Some(serde_json::json!({})),
            created_at: ts,
            team_id: None,
        };
        insert_audit_logs(&mut conn, &[row("q1", t0), row("q2", t0 + 5)]).expect("insert audit");

        let filter = AuditListQuery {
            start_time: Some(t0 - 10),
            end_time: Some(t0 + 10_000),
            user_id: None,
            token_id: None,
            channel_id: None,
            model: None,
            status_code: None,
            keyword: None,
            app_id: None,
            thread_id: None,
            finish_reason: None,
            min_prompt_tokens: None,
            max_prompt_tokens: None,
            min_completion_tokens: None,
            max_completion_tokens: None,
            limit: Some(20),
            offset: Some(0),
        };
        let (threads, total) =
            query_audit_threads(&conn, &filter, AuditConsoleScope::Personal(7)).expect("threads");
        assert_eq!(total, 1);
        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].thread_id, "thread-a");
        assert_eq!(threads[0].request_count, 2);
        assert_eq!(threads[0].total_prompt_tokens, 2);
        assert_eq!(threads[0].total_completion_tokens, 8);
        assert_eq!(threads[0].total_tokens, 10);
        assert_eq!(threads[0].total_cached_prompt_tokens, 0);
        assert!((threads[0].total_cost - 0.04).abs() < 1e-9);
        assert_eq!(threads[0].error_count, 0);
    }

    #[test]
    fn query_audit_threads_counts_http_errors() {
        use crate::audit::{AuditListQuery, AuditRecord};

        let mut conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");
        let t0 = 10_000_000_i64;
        let ok = |rid: &str, ts: i64, code: i64| AuditRecord {
            request_id: rid.into(),
            user_id: Some(7),
            token_id: Some(1),
            channel_id: None,
            model: Some("m".into()),
            request_type: Some("chat".into()),
            request_body_path: None,
            response_body_path: None,
            status_code: Some(code),
            error_message: None,
            prompt_tokens: Some(1),
            completion_tokens: Some(1),
            cached_prompt_tokens: None,
            total_tokens: Some(2),
            cost: Some(0.01),
            latency_ms: Some(10),
            reasoning_phase_ms: None,
            app_id: None,
            thread_id: Some("thread-err".into()),
            finish_reason: None,
            metadata: Some(serde_json::json!({})),
            created_at: ts,
            team_id: None,
        };
        insert_audit_logs(&mut conn, &[ok("a", t0, 200), ok("b", t0 + 1, 500)]).expect("insert");

        let filter = AuditListQuery {
            start_time: Some(t0 - 10),
            end_time: Some(t0 + 10_000),
            user_id: None,
            token_id: None,
            channel_id: None,
            model: None,
            status_code: None,
            keyword: None,
            app_id: None,
            thread_id: None,
            finish_reason: None,
            min_prompt_tokens: None,
            max_prompt_tokens: None,
            min_completion_tokens: None,
            max_completion_tokens: None,
            limit: Some(20),
            offset: Some(0),
        };
        let (threads, total) =
            query_audit_threads(&conn, &filter, AuditConsoleScope::Personal(7)).expect("threads");
        assert_eq!(total, 1);
        assert_eq!(threads[0].error_count, 1);
        assert_eq!(threads[0].request_count, 2);
    }

    #[test]
    fn insert_audit_logs_replace_same_request_id_does_not_double_count_thread() {
        use crate::audit::AuditRecord;

        let mut conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");
        let t0 = 10_000_000_i64;
        let row = AuditRecord {
            request_id: "same-req".into(),
            user_id: Some(7),
            token_id: Some(1),
            channel_id: None,
            model: Some("m".into()),
            request_type: Some("chat".into()),
            request_body_path: None,
            response_body_path: None,
            status_code: Some(200),
            error_message: None,
            prompt_tokens: Some(1),
            completion_tokens: Some(1),
            cached_prompt_tokens: None,
            total_tokens: Some(2),
            cost: Some(0.01),
            latency_ms: Some(10),
            reasoning_phase_ms: None,
            app_id: None,
            thread_id: Some("t1".into()),
            finish_reason: None,
            metadata: None,
            created_at: t0,
            team_id: None,
        };
        insert_audit_logs(&mut conn, std::slice::from_ref(&row)).expect("first insert");
        let mut updated = row;
        updated.status_code = Some(500);
        insert_audit_logs(&mut conn, &[updated]).expect("replace");

        let count: i64 = conn
            .query_row(
                "SELECT request_count FROM audit_threads WHERE user_id = 7 AND team_scope = 0 AND thread_id = 't1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn query_audit_analytics_sums_requests_and_tokens() {
        use crate::audit::{AuditListQuery, AuditRecord};

        let mut conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");
        let t0 = 10_000_000_i64;
        insert_audit_logs(
            &mut conn,
            &[
                AuditRecord {
                    request_id: "q1".into(),
                    user_id: Some(7),
                    token_id: Some(1),
                    channel_id: None,
                    model: Some("m-a".into()),
                    request_type: Some("chat".into()),
                    request_body_path: None,
                    response_body_path: None,
                    status_code: Some(200),
                    error_message: None,
                    prompt_tokens: Some(1),
                    completion_tokens: Some(4),
                    cached_prompt_tokens: None,
                    total_tokens: Some(5),
                    cost: Some(0.02),
                    latency_ms: Some(50),
                    reasoning_phase_ms: None,
                    app_id: None,
                    thread_id: None,
                    finish_reason: None,
                    metadata: None,
                    created_at: t0,
                    team_id: None,
                },
                AuditRecord {
                    request_id: "q2".into(),
                    user_id: Some(7),
                    token_id: Some(1),
                    channel_id: None,
                    model: Some("m-b".into()),
                    request_type: Some("chat".into()),
                    request_body_path: None,
                    response_body_path: None,
                    status_code: Some(500),
                    error_message: None,
                    prompt_tokens: Some(1),
                    completion_tokens: Some(9),
                    cached_prompt_tokens: None,
                    total_tokens: Some(10),
                    cost: None,
                    latency_ms: Some(200),
                    reasoning_phase_ms: None,
                    app_id: None,
                    thread_id: None,
                    finish_reason: None,
                    metadata: None,
                    created_at: t0 + 4000,
                    team_id: None,
                },
            ],
        )
        .expect("insert audit");

        let filter = AuditListQuery {
            start_time: Some(t0 - 10),
            end_time: Some(t0 + 10_000),
            user_id: None,
            token_id: None,
            channel_id: None,
            model: None,
            status_code: None,
            keyword: None,
            app_id: None,
            thread_id: None,
            finish_reason: None,
            min_prompt_tokens: None,
            max_prompt_tokens: None,
            min_completion_tokens: None,
            max_completion_tokens: None,
            limit: None,
            offset: None,
        };

        let resp = query_audit_analytics(&conn, &filter, AuditConsoleScope::Personal(7))
            .expect("analytics");
        assert_eq!(resp.summary.total_requests, 2);
        assert_eq!(resp.summary.success_requests, 1);
        assert_eq!(resp.summary.total_tokens, 15);
        assert!((resp.summary.total_cost - 0.02).abs() < 1e-9);
        assert_eq!(resp.by_model.len(), 2);
        let series_cost: f64 = resp.series.iter().map(|b| b.total_cost).sum();
        assert!((series_cost - 0.02).abs() < 1e-9);
    }

    #[test]
    fn query_audit_analytics_series_includes_prompt_cached_breakdown() {
        use crate::audit::{AuditListQuery, AuditRecord};

        let mut conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");
        let t0 = 10_000_000_i64;
        insert_audit_logs(
            &mut conn,
            &[AuditRecord {
                request_id: "cached1".into(),
                user_id: Some(7),
                token_id: Some(1),
                channel_id: None,
                model: Some("m".into()),
                request_type: Some("chat".into()),
                request_body_path: None,
                response_body_path: None,
                status_code: Some(200),
                error_message: None,
                prompt_tokens: Some(100),
                completion_tokens: Some(50),
                cached_prompt_tokens: Some(80),
                total_tokens: Some(150),
                cost: Some(0.001),
                latency_ms: Some(10),
                reasoning_phase_ms: None,
                app_id: None,
                thread_id: None,
                finish_reason: None,
                metadata: None,
                created_at: t0,
                team_id: None,
            }],
        )
        .expect("insert audit");

        let filter = AuditListQuery {
            start_time: Some(t0 - 10),
            end_time: Some(t0 + 10_000),
            user_id: None,
            token_id: None,
            channel_id: None,
            model: None,
            status_code: None,
            keyword: None,
            app_id: None,
            thread_id: None,
            finish_reason: None,
            min_prompt_tokens: None,
            max_prompt_tokens: None,
            min_completion_tokens: None,
            max_completion_tokens: None,
            limit: None,
            offset: None,
        };

        let resp = query_audit_analytics(&conn, &filter, AuditConsoleScope::Personal(7))
            .expect("analytics");
        let bucket = resp
            .series
            .iter()
            .find(|b| b.bucket_start == (t0 / 3600) * 3600)
            .expect("hour bucket");
        assert_eq!(bucket.prompt_tokens, 100);
        assert_eq!(bucket.completion_tokens, 50);
        assert_eq!(bucket.cached_prompt_tokens, 80);
        assert!((bucket.total_cost - 0.001).abs() < 1e-12);
    }

    #[test]
    fn query_audit_analytics_user_owned_includes_team_key_traffic() {
        use crate::audit::{AuditListQuery, AuditRecord};

        let mut conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");
        let t0 = 20_000_000_i64;
        let uid = create_user(&conn, "team-key-owner", t0).expect("create user");
        let team_id = insert_team(&conn, "Team", "team-slug", uid, t0).expect("team");
        insert_audit_logs(
            &mut conn,
            &[AuditRecord {
                request_id: "team-req".into(),
                user_id: Some(uid),
                token_id: Some(1),
                channel_id: None,
                model: Some("gpt".into()),
                request_type: Some("chat".into()),
                request_body_path: None,
                response_body_path: None,
                status_code: Some(200),
                error_message: None,
                prompt_tokens: Some(3),
                completion_tokens: Some(1),
                cached_prompt_tokens: None,
                total_tokens: Some(4),
                cost: Some(0.07),
                latency_ms: Some(12),
                reasoning_phase_ms: None,
                app_id: None,
                thread_id: None,
                finish_reason: None,
                metadata: None,
                created_at: t0,
                team_id: Some(team_id),
            }],
        )
        .expect("insert audit");

        let filter = AuditListQuery {
            start_time: Some(t0 - 10),
            end_time: Some(t0 + 10_000),
            user_id: None,
            token_id: None,
            channel_id: None,
            model: None,
            status_code: None,
            keyword: None,
            app_id: None,
            thread_id: None,
            finish_reason: None,
            min_prompt_tokens: None,
            max_prompt_tokens: None,
            min_completion_tokens: None,
            max_completion_tokens: None,
            limit: None,
            offset: None,
        };

        let personal = query_audit_analytics(&conn, &filter, AuditConsoleScope::Personal(uid))
            .expect("analytics");
        assert_eq!(
            personal.summary.total_requests, 0,
            "personal scope excludes team-key rows"
        );

        let combined =
            query_audit_analytics(&conn, &filter, AuditConsoleScope::UserOwnedTraffic(uid))
                .expect("analytics");
        assert_eq!(combined.summary.total_requests, 1);
        assert!((combined.summary.total_cost - 0.07).abs() < 1e-12);
    }

    #[test]
    fn billing_deposit_and_charge_usage_ledger() {
        let conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");
        let t = now_secs();
        let user_id = create_user(&conn, "payer", t).expect("create user");
        assert_eq!(get_balance_minor(&conn, user_id).unwrap(), 0);

        let add = crate::money::usd_to_minor(rust_decimal::Decimal::ONE);
        let after_dep = billing_deposit(&conn, user_id, add, t, Some("ext:1")).expect("deposit");
        assert_eq!(after_dep, add);
        assert_eq!(get_balance_minor(&conn, user_id).unwrap(), add);

        let charge = crate::money::usd_to_minor(Decimal::from_str("0.1").unwrap());
        let after_use = billing_charge_usage(
            &conn,
            user_id,
            None,
            charge,
            t + 1,
            BillingUsageChargeMeta {
                request_id: "req-a",
                model: Some("m1"),
                prompt_tokens: Some(1),
                completion_tokens: Some(2),
            },
        )
        .expect("charge");
        assert_eq!(after_use, add - charge);

        let deposits = list_billing_ledger(&conn, user_id, Some("deposit"), 10, 0).unwrap();
        assert_eq!(deposits.len(), 1);
        assert_eq!(deposits[0].kind, "deposit");

        let usage = list_billing_ledger(&conn, user_id, Some("usage_charge"), 10, 0).unwrap();
        assert_eq!(usage.len(), 1);
        assert_eq!(usage[0].request_id.as_deref(), Some("req-a"));
    }

    #[test]
    fn billing_charge_insufficient_returns_error() {
        let conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");
        let t = now_secs();
        let user_id = create_user(&conn, "broke", t).expect("create user");
        let err = billing_charge_usage(
            &conn,
            user_id,
            None,
            1000,
            t,
            BillingUsageChargeMeta {
                request_id: "req-b",
                model: None,
                prompt_tokens: None,
                completion_tokens: None,
            },
        )
        .expect_err("insufficient");
        match err {
            BillingChargeError::Insufficient { balance_minor } => assert_eq!(balance_minor, 0),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn billing_charge_zero_skips_ledger() {
        let conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");
        let t = now_secs();
        let user_id = create_user(&conn, "noop", t).expect("create user");
        let bal = billing_charge_usage(
            &conn,
            user_id,
            None,
            0,
            t,
            BillingUsageChargeMeta {
                request_id: "req-c",
                model: None,
                prompt_tokens: None,
                completion_tokens: None,
            },
        )
        .unwrap();
        assert_eq!(bal, 0);
        let all = list_billing_ledger(&conn, user_id, None, 20, 0).unwrap();
        assert!(all.is_empty());
    }

    fn read_key_spend_state(
        conn: &Connection,
        key_id: i64,
    ) -> rusqlite::Result<(Option<String>, String, Option<i64>)> {
        conn.query_row(
            "SELECT quota_monthly_spend_minor, quota_used_spend_minor, quota_spend_period_start FROM api_keys WHERE id = ?1",
            params![key_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
    }

    /// Fixed “mid-month” instant so `unix_month_start` is deterministic across platforms.
    fn ts_utc(year: i32, month: u32, day: u32) -> i64 {
        Utc.with_ymd_and_hms(year, month, day, 15, 0, 0)
            .unwrap()
            .timestamp()
    }

    #[test]
    fn ensure_monthly_spend_no_cap_ok() {
        let conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");
        let t = ts_utc(2026, 4, 10);
        let uid = create_user(&conn, "spend-a", t).expect("user");
        let kid = insert_api_key_with_meta(
            &conn,
            uid,
            "sk-or-v1-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            t,
            "k",
            "",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("key");
        assert!(ensure_monthly_spend_quota(&conn, kid, t).is_ok());
    }

    #[test]
    fn ensure_monthly_spend_invalid_cap_ok() {
        let conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");
        let t = ts_utc(2026, 4, 10);
        let uid = create_user(&conn, "spend-b", t).expect("user");
        let kid =
            insert_api_key_for_user(&conn, uid, "sk-or-v1-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", t)
                .expect("key");
        conn.execute(
            "UPDATE api_keys SET quota_monthly_spend_minor = '0', quota_used_spend_minor = '999' WHERE id = ?1",
            params![kid],
        )
        .expect("update");
        assert!(ensure_monthly_spend_quota(&conn, kid, t).is_ok());
    }

    #[test]
    fn ensure_monthly_spend_within_and_exceeded() {
        let conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");
        let t = ts_utc(2026, 4, 10);
        let uid = create_user(&conn, "spend-c", t).expect("user");
        let kid = insert_api_key_with_meta(
            &conn,
            uid,
            "sk-or-v1-cccccccccccccccccccccccccccccccc",
            t,
            "k",
            "",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(1000),
        )
        .expect("key");
        assert!(ensure_monthly_spend_quota(&conn, kid, t).is_ok());

        conn.execute(
            "UPDATE api_keys SET quota_used_spend_minor = ?1 WHERE id = ?2",
            params![crate::money::minor_to_db(1000_i128), kid],
        )
        .expect("set used");
        assert_eq!(
            ensure_monthly_spend_quota(&conn, kid, t),
            Err("monthly spend quota exceeded")
        );
    }

    #[test]
    fn ensure_monthly_spend_resets_when_month_changes() {
        let conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");
        let now = ts_utc(2026, 3, 15);
        let month_start = unix_month_start(now);
        let prev_month_start = unix_month_start(month_start - 1);

        let uid = create_user(&conn, "spend-d", now).expect("user");
        let kid = insert_api_key_with_meta(
            &conn,
            uid,
            "sk-or-v1-dddddddddddddddddddddddddddddddd",
            now,
            "k",
            "",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(10_000),
        )
        .expect("key");

        conn.execute(
            "UPDATE api_keys SET quota_used_spend_minor = ?1, quota_spend_period_start = ?2 WHERE id = ?3",
            params![
                crate::money::minor_to_db(5000_i128),
                prev_month_start,
                kid
            ],
        )
        .expect("backdate period");

        assert!(ensure_monthly_spend_quota(&conn, kid, now).is_ok());

        let (_, used, period) = read_key_spend_state(&conn, kid).expect("read");
        assert_eq!(used, "0");
        assert_eq!(period, Some(month_start));
    }

    #[test]
    fn billing_charge_increments_key_spend_when_gateway_id_set() {
        let conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");
        let t = ts_utc(2026, 5, 1);
        let user_id = create_user(&conn, "spend-e", t).expect("user");
        let add = crate::money::usd_to_minor(Decimal::ONE);
        billing_deposit(&conn, user_id, add, t, None).expect("deposit");

        let key_id = insert_api_key_with_meta(
            &conn,
            user_id,
            "sk-or-v1-eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            t,
            "k",
            "",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(500_000_000_000_000_000),
        )
        .expect("key");

        let charge = 42_i128;
        let meta = BillingUsageChargeMeta {
            request_id: "req-spend-1",
            model: Some("m"),
            prompt_tokens: Some(1),
            completion_tokens: Some(2),
        };
        let after = billing_charge_usage(&conn, user_id, Some(key_id), charge, t + 1, meta)
            .expect("charge");
        assert_eq!(after, add - charge);

        let (_, used_s, _) = read_key_spend_state(&conn, key_id).expect("read");
        assert_eq!(crate::money::minor_from_db(&used_s).unwrap(), charge);
    }

    #[test]
    fn billing_charge_does_not_touch_key_spend_without_cap() {
        let conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");
        let t = ts_utc(2026, 5, 2);
        let user_id = create_user(&conn, "spend-f", t).expect("user");
        let add = crate::money::usd_to_minor(Decimal::ONE);
        billing_deposit(&conn, user_id, add, t, None).expect("deposit");

        let key_id = insert_api_key_for_user(
            &conn,
            user_id,
            "sk-or-v1-ffffffffffffffffffffffffffffffff",
            t,
        )
        .expect("key");

        let charge = 100_i128;
        billing_charge_usage(
            &conn,
            user_id,
            Some(key_id),
            charge,
            t + 1,
            BillingUsageChargeMeta {
                request_id: "req-spend-2",
                model: None,
                prompt_tokens: None,
                completion_tokens: None,
            },
        )
        .expect("charge");

        let (_, used_s, _) = read_key_spend_state(&conn, key_id).expect("read");
        assert_eq!(used_s, "0");
    }

    #[test]
    fn billing_charge_insufficient_leaves_key_spend_unchanged() {
        let conn = Connection::open_in_memory().expect("open db");
        run_migrations(&conn).expect("migration");
        let t = ts_utc(2026, 5, 3);
        let user_id = create_user(&conn, "spend-g", t).expect("user");
        assert_eq!(get_balance_minor(&conn, user_id).unwrap(), 0);

        let key_id = insert_api_key_with_meta(
            &conn,
            user_id,
            "sk-or-v1-gggggggggggggggggggggggggggggggg",
            t,
            "k",
            "",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(1_000_000),
        )
        .expect("key");

        conn.execute(
            "UPDATE api_keys SET quota_used_spend_minor = ?1 WHERE id = ?2",
            params![crate::money::minor_to_db(100_i128), key_id],
        )
        .expect("set used");

        let err = billing_charge_usage(
            &conn,
            user_id,
            Some(key_id),
            500,
            t,
            BillingUsageChargeMeta {
                request_id: "req-spend-fail",
                model: None,
                prompt_tokens: None,
                completion_tokens: None,
            },
        )
        .expect_err("insufficient");

        assert!(matches!(err, BillingChargeError::Insufficient { .. }));

        let (_, used_s, _) = read_key_spend_state(&conn, key_id).expect("read");
        assert_eq!(crate::money::minor_from_db(&used_s).unwrap(), 100);
    }
}
