use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

const JWT_EXPIRY_SECS: i64 = 7 * 24 * 3600;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    username: String,
    role: String,
    exp: i64,
}

pub struct SessionClaims {
    pub user_id: i64,
    pub username: String,
    pub role: String,
}

pub fn encode_session_jwt(
    secret: &str,
    user_id: i64,
    username: &str,
    role: &str,
) -> Result<String, String> {
    let now = Utc::now().timestamp();
    let exp = now + JWT_EXPIRY_SECS;
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| e.to_string())
}

pub fn decode_session_jwt(secret: &str, token: &str) -> Result<SessionClaims, String> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| e.to_string())?;
    let sub: i64 = data
        .claims
        .sub
        .parse()
        .map_err(|_| "invalid subject".to_string())?;
    Ok(SessionClaims {
        user_id: sub,
        username: data.claims.username,
        role: data.claims.role,
    })
}
