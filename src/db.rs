use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex, MutexGuard};

const MIGRATIONS: [(&str, &str); 1] = [(
    "0001_create_users.sql",
    include_str!("../migrations/0001_create_users.sql"),
)];

pub type DbConn = Arc<Mutex<Connection>>;

pub fn lock_db(db: &DbConn) -> MutexGuard<'_, Connection> {
    db.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
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

pub fn validate_api_key(conn: &Connection, api_key: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM api_keys WHERE api_key = ?1 AND revoked = 0",
        params![api_key],
        |_| Ok(()),
    )
    .is_ok()
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
