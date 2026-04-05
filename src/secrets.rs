//! Helpers for logging secrets without printing full values.

use sha2::{Digest, Sha256};

/// Mask an API key for logs: visible prefix, `...`, last 4 characters, and total length.
pub fn mask_secret(s: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        return "(empty)".to_string();
    }
    let len = t.len();
    if len <= 8 {
        return format!("*** (len={len})");
    }
    let head_len = 14.min(len.saturating_sub(4));
    let head = &t[..head_len];
    let tail = &t[len - 4..];
    format!("{head}...{tail} (len={len})")
}

/// SHA-256 hex digest of the secret (compare with local `sha256` / PowerShell without pasting the key).
pub fn secret_sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.trim().as_bytes());
    hex::encode(h.finalize())
}

/// SHA-256 hex of the full API key string (for lookup; never log the raw key).
pub fn api_key_sha256_hex(full_key: &str) -> String {
    secret_sha256_hex(full_key)
}

/// Short preview for UI (first 12 + ellipsis + last 4).
pub fn api_key_preview_short(full: &str) -> String {
    let b = full.as_bytes();
    if b.len() <= 14 {
        return "••••".to_string();
    }
    let start = std::str::from_utf8(&b[..12]).unwrap_or("••••");
    let end = std::str::from_utf8(&b[b.len() - 4..]).unwrap_or("");
    format!("{start}…{end}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_secret_shows_prefix_suffix_and_len() {
        let m = mask_secret("sk-or-v1-abcdef0123456789");
        assert!(m.starts_with("sk-or-v1-abcde"));
        assert!(m.contains("6789"));
        assert!(m.contains("len=25"));
    }

    #[test]
    fn secret_sha256_hex_is_stable() {
        let h = secret_sha256_hex("hello");
        assert_eq!(h.len(), 64);
        assert_eq!(
            h,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}
