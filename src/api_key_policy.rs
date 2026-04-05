//! Model / IP allowlists and quota window helpers for API keys.

use std::net::IpAddr;
use std::str::FromStr;

use actix_web::HttpRequest;
use chrono::{Datelike, TimeZone, Utc};
use ipnetwork::IpNetwork;

/// UTC midnight at the first day of the calendar month containing `ts`.
pub fn unix_month_start(ts: i64) -> i64 {
    let dt = Utc
        .timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(Utc::now);
    Utc.with_ymd_and_hms(dt.year(), dt.month(), 1, 0, 0, 0)
        .unwrap()
        .timestamp()
}

pub fn check_model_allowlist(
    model_allowlist_json: Option<&str>,
    model: Option<&str>,
) -> Result<(), &'static str> {
    let Some(raw) = model_allowlist_json.filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    let arr: Vec<String> =
        serde_json::from_str(raw).map_err(|_| "invalid model_allowlist JSON")?;
    if arr.is_empty() {
        return Ok(());
    }
    let Some(m) = model else {
        return Err("model is required for this API key");
    };
    if arr.iter().any(|a| a == m) {
        Ok(())
    } else {
        Err("model is not allowed for this API key")
    }
}

/// Prefer `X-Forwarded-For` first hop, else socket peer IP (v4/v6).
pub fn client_ip(req: &HttpRequest) -> Option<IpAddr> {
    if let Some(xff) = req.headers().get("x-forwarded-for") {
        if let Ok(s) = xff.to_str() {
            if let Some(first) = s.split(',').next().map(str::trim).filter(|s| !s.is_empty()) {
                if let Ok(ip) = first.parse::<IpAddr>() {
                    return Some(ip);
                }
            }
        }
    }
    req.peer_addr().map(|a| a.ip())
}

pub fn check_ip_allowlist(
    ip_allowlist_json: Option<&str>,
    client_ip: IpAddr,
) -> Result<(), &'static str> {
    let Some(raw) = ip_allowlist_json.filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    let arr: Vec<String> = serde_json::from_str(raw).map_err(|_| "invalid ip_allowlist JSON")?;
    if arr.is_empty() {
        return Ok(());
    }
    for cidr in &arr {
        let net = IpNetwork::from_str(cidr.trim()).map_err(|_| "invalid CIDR in ip_allowlist")?;
        if net.contains(client_ip) {
            return Ok(());
        }
    }
    Err("client IP is not allowed for this API key")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_allowlist_allows_subset() {
        let j = r#"["gpt-4","gpt-3.5-turbo"]"#;
        assert!(check_model_allowlist(Some(j), Some("gpt-4")).is_ok());
        assert!(check_model_allowlist(Some(j), Some("gpt-3.5-turbo")).is_ok());
        assert!(check_model_allowlist(Some(j), Some("other")).is_err());
    }

    #[test]
    fn ip_allowlist_cidr() {
        let j = r#"["127.0.0.0/8","::1/128"]"#;
        assert!(check_ip_allowlist(
            Some(j),
            "127.0.0.1".parse().unwrap()
        )
        .is_ok());
    }
}
