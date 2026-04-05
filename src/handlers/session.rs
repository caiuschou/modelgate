use actix_web::{web, HttpResponse};
use bcrypt::{hash, verify, DEFAULT_COST};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{errors::ApiError, jwt_session, AppState};

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub invite_code: String,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub username: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: LoginUserDto,
}

#[derive(Serialize)]
pub struct LoginUserDto {
    pub username: String,
    pub role: String,
}

fn create_api_key() -> String {
    let mut rng = rand::thread_rng();
    let random_part: String = (0..32)
        .map(|_| format!("{:x}", rng.gen::<u8>() % 16))
        .collect();
    format!("sk-or-v1-{}", random_part)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn validate_username(username: &str) -> Result<&str, ApiError> {
    let u = username.trim();
    if u.is_empty() {
        return Err(ApiError::BadRequest("username is required".into()));
    }
    if u.len() > 64 {
        return Err(ApiError::BadRequest(
            "username must be at most 64 characters".into(),
        ));
    }
    Ok(u)
}

pub async fn register(
    state: web::Data<AppState>,
    req: web::Json<RegisterRequest>,
) -> Result<HttpResponse, ApiError> {
    let username = validate_username(&req.username)?;

    let expected = state.cfg.auth.invite_code.trim();
    if expected.is_empty() {
        return Err(ApiError::BadRequest(
            "Self-service registration is disabled".into(),
        ));
    }
    if req.invite_code.trim() != expected {
        return Err(ApiError::BadRequest("Invalid invite code".into()));
    }

    let password_hash = hash(&req.password, DEFAULT_COST)
        .map_err(|_| ApiError::InternalError("password hashing failed".into()))?;

    let api_key = create_api_key();
    let created_at = now_secs();

    state
        .user_service
        .register_user_with_password_and_api_key(username, &password_hash, &api_key, created_at)
        .map_err(ApiError::from)?;

    Ok(HttpResponse::Created().json(RegisterResponse {
        username: username.to_string(),
    }))
}

pub async fn login(
    state: web::Data<AppState>,
    req: web::Json<LoginRequest>,
) -> Result<HttpResponse, ApiError> {
    let username = validate_username(&req.username)?;

    let creds = state
        .user_service
        .get_user_login_credentials(username)
        .map_err(ApiError::from)?;

    let (user_id, stored_hash) = match creds {
        Some((id, Some(h))) => (id, h),
        _ => {
            return Err(ApiError::Unauthorized(
                "Invalid username or password".into(),
            ))
        }
    };

    let ok = verify(&req.password, &stored_hash).unwrap_or(false);
    if !ok {
        return Err(ApiError::Unauthorized(
            "Invalid username or password".into(),
        ));
    }

    let role = if username.eq_ignore_ascii_case("admin") {
        "admin"
    } else {
        "user"
    };

    let token = jwt_session::encode_session_jwt(
        &state.cfg.auth.jwt_secret,
        user_id,
        username,
        role,
    )
    .map_err(|_| ApiError::InternalError("session token failed".into()))?;

    Ok(HttpResponse::Ok().json(LoginResponse {
        token,
        user: LoginUserDto {
            username: username.to_string(),
            role: role.to_string(),
        },
    }))
}
