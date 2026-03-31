use actix_web::HttpRequest;
use rusqlite::Connection;

pub fn extract_bearer_token(req: &HttpRequest) -> Option<&str> {
    req.headers()
        .get(actix_web::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

pub fn validate_api_key(conn: &Connection, api_key: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM api_keys WHERE api_key = ?1 AND revoked = 0",
        rusqlite::params![api_key],
        |_| Ok(()),
    )
    .is_ok()
}
