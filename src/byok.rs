//! Bring-your-own-key: encrypt upstream API keys at rest and resolve profiles for chat proxy.

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use rand::RngCore;
use rusqlite::{params, Connection};
use serde::Serialize;

const NONCE_LEN: usize = 12;

#[derive(Debug)]
pub enum ByokCryptoError {
    EncryptFailed,
    DecryptFailed,
    BadBlob,
}

/// Seal `plaintext` with AES-256-GCM. Output: `nonce || ciphertext` (ciphertext includes auth tag).
pub fn seal_upstream_api_key(
    master_key: &[u8; 32],
    plaintext: &[u8],
) -> Result<Vec<u8>, ByokCryptoError> {
    let key = Key::<Aes256Gcm>::from_slice(master_key.as_slice());
    let cipher = Aes256Gcm::new(key);
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let mut ct = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| ByokCryptoError::EncryptFailed)?;
    let mut blob = Vec::with_capacity(NONCE_LEN + ct.len());
    blob.extend_from_slice(&nonce_bytes);
    blob.append(&mut ct);
    Ok(blob)
}

pub fn open_upstream_api_key(
    master_key: &[u8; 32],
    blob: &[u8],
) -> Result<Vec<u8>, ByokCryptoError> {
    if blob.len() <= NONCE_LEN {
        return Err(ByokCryptoError::BadBlob);
    }
    let key = Key::<Aes256Gcm>::from_slice(master_key.as_slice());
    let cipher = Aes256Gcm::new(key);
    let (nonce_b, ct) = blob.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_b);
    cipher
        .decrypt(nonce, ct)
        .map_err(|_| ByokCryptoError::DecryptFailed)
}

pub fn split_sealed_blob(blob: &[u8]) -> Result<(&[u8], &[u8]), ByokCryptoError> {
    if blob.len() <= NONCE_LEN {
        return Err(ByokCryptoError::BadBlob);
    }
    Ok(blob.split_at(NONCE_LEN))
}

pub fn join_nonce_ciphertext(nonce: &[u8], ciphertext: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(nonce.len() + ciphertext.len());
    v.extend_from_slice(nonce);
    v.extend_from_slice(ciphertext);
    v
}

#[derive(Debug, Clone, Serialize)]
pub struct ByokProfileSummary {
    pub id: i64,
    pub name: String,
    pub base_url: String,
    pub api_key_preview: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub revoked: bool,
}

#[derive(Debug)]
pub struct ByokResolvedUpstream {
    pub base_url: String,
    pub api_key: String,
    pub profile_id: i64,
}

/// List BYOK profiles for console: personal (`team_id` None) or team (`Some(team_id)`).
pub fn list_byok_profiles(
    conn: &Connection,
    user_id: i64,
    team_id: Option<i64>,
) -> rusqlite::Result<Vec<ByokProfileSummary>> {
    match team_id {
        None => {
            let mut stmt = conn.prepare(
                "SELECT id, name, base_url, api_key_ciphertext, created_at, updated_at, revoked_at
                 FROM byok_profiles
                 WHERE owner_user_id = ?1 AND owner_team_id IS NULL
                 ORDER BY id DESC",
            )?;
            let rows = stmt.query_map(params![user_id], map_summary_row)?;
            rows.collect()
        }
        Some(tid) => {
            let mut stmt = conn.prepare(
                "SELECT id, name, base_url, api_key_ciphertext, created_at, updated_at, revoked_at
                 FROM byok_profiles
                 WHERE owner_team_id = ?1
                 ORDER BY id DESC",
            )?;
            let rows = stmt.query_map(params![tid], map_summary_row)?;
            rows.collect()
        }
    }
}

fn map_summary_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ByokProfileSummary> {
    let id: i64 = row.get(0)?;
    let name: String = row.get(1)?;
    let base_url: String = row.get(2)?;
    let ct: Vec<u8> = row.get(3)?;
    let created_at: i64 = row.get(4)?;
    let updated_at: i64 = row.get(5)?;
    let revoked_at: Option<i64> = row.get(6)?;
    let preview = preview_from_ciphertext_len(ct.len());
    Ok(ByokProfileSummary {
        id,
        name,
        base_url,
        api_key_preview: preview,
        created_at,
        updated_at,
        revoked: revoked_at.is_some(),
    })
}

fn preview_from_ciphertext_len(len: usize) -> String {
    if len == 0 {
        return "••••".to_string();
    }
    format!("sealed({len}B)")
}

pub fn get_byok_profile_detail(
    conn: &Connection,
    id: i64,
    user_id: i64,
    team_id: Option<i64>,
) -> rusqlite::Result<Option<ByokProfileSummary>> {
    let row = match team_id {
        None => conn.query_row(
            "SELECT id, name, base_url, api_key_ciphertext, created_at, updated_at, revoked_at
             FROM byok_profiles
             WHERE id = ?1 AND owner_user_id = ?2 AND owner_team_id IS NULL",
            params![id, user_id],
            map_summary_row,
        ),
        Some(tid) => conn.query_row(
            "SELECT id, name, base_url, api_key_ciphertext, created_at, updated_at, revoked_at
             FROM byok_profiles
             WHERE id = ?1 AND owner_team_id = ?2",
            params![id, tid],
            map_summary_row,
        ),
    };
    match row {
        Ok(s) => Ok(Some(s)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn insert_byok_profile(
    conn: &Connection,
    owner_user_id: Option<i64>,
    owner_team_id: Option<i64>,
    name: &str,
    base_url: &str,
    api_key_nonce: &[u8],
    api_key_ciphertext: &[u8],
    now: i64,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO byok_profiles (
            owner_user_id, owner_team_id, name, base_url,
            api_key_nonce, api_key_ciphertext, key_version, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8)",
        params![
            owner_user_id,
            owner_team_id,
            name,
            base_url,
            api_key_nonce,
            api_key_ciphertext,
            now,
            now,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

#[derive(Debug)]
pub struct ByokProfileStored {
    pub name: String,
    pub base_url: String,
    pub api_key_nonce: Vec<u8>,
    pub api_key_ciphertext: Vec<u8>,
}

pub fn fetch_byok_profile_stored(
    conn: &Connection,
    id: i64,
    user_id: i64,
    team_id: Option<i64>,
) -> rusqlite::Result<Option<ByokProfileStored>> {
    let row = match team_id {
        None => conn.query_row(
            "SELECT name, base_url, api_key_nonce, api_key_ciphertext
             FROM byok_profiles
             WHERE id = ?1 AND owner_user_id = ?2 AND owner_team_id IS NULL AND revoked_at IS NULL",
            params![id, user_id],
            |row| {
                Ok(ByokProfileStored {
                    name: row.get(0)?,
                    base_url: row.get(1)?,
                    api_key_nonce: row.get(2)?,
                    api_key_ciphertext: row.get(3)?,
                })
            },
        ),
        Some(tid) => conn.query_row(
            "SELECT name, base_url, api_key_nonce, api_key_ciphertext
             FROM byok_profiles
             WHERE id = ?1 AND owner_team_id = ?2 AND revoked_at IS NULL",
            params![id, tid],
            |row| {
                Ok(ByokProfileStored {
                    name: row.get(0)?,
                    base_url: row.get(1)?,
                    api_key_nonce: row.get(2)?,
                    api_key_ciphertext: row.get(3)?,
                })
            },
        ),
    };
    match row {
        Ok(s) => Ok(Some(s)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn update_byok_profile_stored(
    conn: &Connection,
    id: i64,
    user_id: i64,
    team_id: Option<i64>,
    stored: &ByokProfileStored,
    now: i64,
) -> rusqlite::Result<usize> {
    match team_id {
        None => conn.execute(
            "UPDATE byok_profiles SET name = ?1, base_url = ?2, api_key_nonce = ?3, api_key_ciphertext = ?4, updated_at = ?5
             WHERE id = ?6 AND owner_user_id = ?7 AND owner_team_id IS NULL AND revoked_at IS NULL",
            params![
                &stored.name,
                &stored.base_url,
                &stored.api_key_nonce,
                &stored.api_key_ciphertext,
                now,
                id,
                user_id,
            ],
        ),
        Some(tid) => conn.execute(
            "UPDATE byok_profiles SET name = ?1, base_url = ?2, api_key_nonce = ?3, api_key_ciphertext = ?4, updated_at = ?5
             WHERE id = ?6 AND owner_team_id = ?7 AND revoked_at IS NULL",
            params![
                &stored.name,
                &stored.base_url,
                &stored.api_key_nonce,
                &stored.api_key_ciphertext,
                now,
                id,
                tid,
            ],
        ),
    }
}

pub fn revoke_byok_profile(
    conn: &Connection,
    id: i64,
    user_id: i64,
    team_id: Option<i64>,
    now: i64,
) -> rusqlite::Result<usize> {
    match team_id {
        None => conn.execute(
            "UPDATE byok_profiles SET revoked_at = ?1, updated_at = ?1
             WHERE id = ?2 AND owner_user_id = ?3 AND owner_team_id IS NULL AND revoked_at IS NULL",
            params![now, id, user_id],
        ),
        Some(tid) => conn.execute(
            "UPDATE byok_profiles SET revoked_at = ?1, updated_at = ?1
             WHERE id = ?2 AND owner_team_id = ?3 AND revoked_at IS NULL",
            params![now, id, tid],
        ),
    }
}

type ByokGatewayRow =
    (String, Vec<u8>, Vec<u8>, Option<i64>, Option<i64>, Option<i64>);

/// Whether a non-revoked profile may be set as `api_keys.default_byok_profile_id` for this key scope.
pub fn profile_bindable_for_gateway_key(
    conn: &Connection,
    profile_id: i64,
    key_user_id: i64,
    key_team_id: Option<i64>,
) -> rusqlite::Result<bool> {
    let row: Result<(Option<i64>, Option<i64>), rusqlite::Error> = conn.query_row(
        "SELECT owner_user_id, owner_team_id FROM byok_profiles WHERE id = ?1 AND revoked_at IS NULL",
        params![profile_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );
    let (owner_user_id, owner_team_id) = match row {
        Ok(r) => r,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
        Err(e) => return Err(e),
    };
    Ok(match key_team_id {
        None => owner_team_id.is_none() && owner_user_id == Some(key_user_id),
        Some(tid) => owner_team_id == Some(tid),
    })
}

/// Clear gateway keys that pointed at a revoked BYOK profile.
pub fn clear_default_byok_refs_for_profile(
    conn: &Connection,
    profile_id: i64,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE api_keys SET default_byok_profile_id = NULL WHERE default_byok_profile_id = ?1",
        params![profile_id],
    )
}

/// Resolve a BYOK profile for gateway forwarding (membership + key team alignment).
pub fn resolve_byok_for_gateway(
    conn: &Connection,
    profile_id: i64,
    gateway_user_id: i64,
    api_key_team_id: Option<i64>,
    master_key: &[u8; 32],
) -> Result<ByokResolvedUpstream, ByokResolveError> {
    let row: Result<ByokGatewayRow, rusqlite::Error> = conn.query_row(
            "SELECT base_url, api_key_nonce, api_key_ciphertext, owner_user_id, owner_team_id, revoked_at
             FROM byok_profiles WHERE id = ?1",
            params![profile_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        );

    let (base_url, nonce, ciphertext, owner_user_id, owner_team_id, revoked_at) = match row {
        Ok(r) => r,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Err(ByokResolveError::NotFound),
        Err(e) => return Err(ByokResolveError::Db(e)),
    };

    if revoked_at.is_some() {
        return Err(ByokResolveError::NotFound);
    }

    let personal = owner_team_id.is_none() && owner_user_id == Some(gateway_user_id);
    let team_match = owner_team_id.is_some() && owner_team_id == api_key_team_id;
    if !personal && !team_match {
        return Err(ByokResolveError::NotFound);
    }

    let joined = join_nonce_ciphertext(&nonce, &ciphertext);
    let plain =
        open_upstream_api_key(master_key, &joined).map_err(|_| ByokResolveError::Decrypt)?;
    let api_key = String::from_utf8(plain).map_err(|_| ByokResolveError::Decrypt)?;

    Ok(ByokResolvedUpstream {
        base_url,
        api_key,
        profile_id,
    })
}

#[derive(Debug)]
pub enum ByokResolveError {
    NotFound,
    Decrypt,
    Db(rusqlite::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_open_roundtrip() {
        let key: [u8; 32] = [7u8; 32];
        let pt = b"sk-secret-upstream-key";
        let blob = seal_upstream_api_key(&key, pt).expect("seal");
        let out = open_upstream_api_key(&key, &blob).expect("open");
        assert_eq!(out.as_slice(), pt.as_slice());
    }

    #[test]
    fn split_join_roundtrip() {
        let key: [u8; 32] = [9u8; 32];
        let blob = seal_upstream_api_key(&key, b"k").unwrap();
        let (n, c) = split_sealed_blob(&blob).unwrap();
        let j = join_nonce_ciphertext(n, c);
        assert_eq!(open_upstream_api_key(&key, &j).unwrap(), b"k");
    }
}
