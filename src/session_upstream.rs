//! Session-level upstream affinity: ordered pool, Round Robin + SQLite bindings.

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum UpstreamPoolEntry {
    Platform,
    Byok { byok_profile_id: i64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickedUpstream {
    Platform,
    Byok(i64),
}

/// Prefer `X-Thread-Id` (already trimmed); else OpenAI `user` in JSON body.
pub fn parse_session_key(thread_id: Option<String>, body: &[u8]) -> Option<String> {
    if let Some(t) = thread_id {
        let s = t.trim();
        if !s.is_empty() {
            return Some(s.to_string());
        }
    }
    let v: serde_json::Value = serde_json::from_slice(body).ok()?;
    let u = v.get("user").and_then(|x| x.as_str())?;
    let u = u.trim();
    if u.is_empty() {
        return None;
    }
    Some(u.to_string())
}

pub fn parse_upstream_pool_json(raw: Option<&str>) -> Result<Vec<UpstreamPoolEntry>, &'static str> {
    let Some(s) = raw.filter(|x| !x.trim().is_empty()) else {
        return Ok(vec![]);
    };
    let v: Vec<UpstreamPoolEntry> =
        serde_json::from_str(s).map_err(|_| "invalid upstream_pool_json")?;
    validate_upstream_pool(&v)?;
    Ok(v)
}

pub fn validate_upstream_pool(pool: &[UpstreamPoolEntry]) -> Result<(), &'static str> {
    if pool.is_empty() {
        return Err("upstream pool must be non-empty");
    }
    let mut platform_count = 0usize;
    let mut seen = std::collections::HashSet::new();
    for e in pool {
        match e {
            UpstreamPoolEntry::Platform => {
                platform_count += 1;
                if platform_count > 1 {
                    return Err("upstream pool may include at most one platform entry");
                }
            }
            UpstreamPoolEntry::Byok { byok_profile_id } => {
                if *byok_profile_id <= 0 {
                    return Err("byok_profile_id must be positive");
                }
                if !seen.insert(*byok_profile_id) {
                    return Err("upstream pool BYOK entries must be unique");
                }
            }
        }
    }
    Ok(())
}

fn row_to_pick(kind: &str, byok_profile_id: Option<i64>) -> PickedUpstream {
    match kind {
        "platform" => PickedUpstream::Platform,
        "byok" => {
            PickedUpstream::Byok(byok_profile_id.expect("byok binding row must have profile id"))
        }
        _ => panic!("corrupt session_upstream_bindings.kind"),
    }
}

/// Returns chosen upstream for this session (existing binding or RR + new binding).
pub fn pick_session_upstream(
    conn: &mut Connection,
    key_id: i64,
    pool: &[UpstreamPoolEntry],
    session_key: &str,
    now: i64,
) -> rusqlite::Result<PickedUpstream> {
    debug_assert!(!pool.is_empty());
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let existing: Option<(String, Option<i64>)> = tx
        .query_row(
            "SELECT kind, byok_profile_id FROM session_upstream_bindings
         WHERE api_key_id = ?1 AND session_key = ?2",
            params![key_id, session_key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    if let Some((kind, pid)) = existing {
        let pick = row_to_pick(&kind, pid);
        tx.commit()?;
        return Ok(pick);
    }

    let cur: i64 = tx.query_row(
        "SELECT session_rr_cursor FROM api_keys WHERE id = ?1",
        params![key_id],
        |row| row.get(0),
    )?;
    let n = pool.len() as i64;
    let idx = ((cur % n) + n) % n;
    let entry = pool[idx as usize].clone();
    let (kind_s, pid) = match &entry {
        UpstreamPoolEntry::Platform => ("platform", None::<i64>),
        UpstreamPoolEntry::Byok { byok_profile_id } => ("byok", Some(*byok_profile_id)),
    };

    let inserted = tx.execute(
        "INSERT OR IGNORE INTO session_upstream_bindings (api_key_id, session_key, kind, byok_profile_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![key_id, session_key, kind_s, pid, now],
    )?;
    if inserted == 0 {
        let (kind, bpid): (String, Option<i64>) = tx.query_row(
            "SELECT kind, byok_profile_id FROM session_upstream_bindings
             WHERE api_key_id = ?1 AND session_key = ?2",
            params![key_id, session_key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let pick = row_to_pick(&kind, bpid);
        tx.commit()?;
        return Ok(pick);
    }

    let next = (cur + 1) % n;
    tx.execute(
        "UPDATE api_keys SET session_rr_cursor = ?1 WHERE id = ?2",
        params![next, key_id],
    )?;
    let pick = match entry {
        UpstreamPoolEntry::Platform => PickedUpstream::Platform,
        UpstreamPoolEntry::Byok { byok_profile_id } => PickedUpstream::Byok(byok_profile_id),
    };
    tx.commit()?;
    Ok(pick)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_session_key_prefers_thread() {
        let sk = parse_session_key(Some("  t1  ".into()), br#"{"user":"u1","model":"x"}"#);
        assert_eq!(sk.as_deref(), Some("t1"));
    }

    #[test]
    fn parse_session_key_falls_back_to_user() {
        let sk = parse_session_key(None, br#"{"user":" end ","model":"x"}"#);
        assert_eq!(sk.as_deref(), Some("end"));
    }

    #[test]
    fn validate_pool_rejects_dup_byok() {
        let p = vec![
            UpstreamPoolEntry::Platform,
            UpstreamPoolEntry::Byok { byok_profile_id: 1 },
            UpstreamPoolEntry::Byok { byok_profile_id: 1 },
        ];
        assert!(validate_upstream_pool(&p).is_err());
    }
}
