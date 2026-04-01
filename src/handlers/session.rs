use actix_web::{web, HttpResponse};
use bcrypt::{hash, verify, DEFAULT_COST};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{errors::ApiError, AppState};

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
    let random_part: String = (0..32).map(|_| format!("{:x}", rng.gen::<u8>() % 16)).collect();
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

fn validate_password(password: &str) -> Result<(), ApiError> {
    if password.len() < 8 {
        return Err(ApiError::BadRequest(
            "password must be at least 8 characters".into(),
        ));
    }
    let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    if !has_upper || !has_lower || !has_digit {
        return Err(ApiError::BadRequest(
            "password must include uppercase, lowercase, and a digit".into(),
        ));
    }
    Ok(())
}

pub async fn register(
    state: web::Data<AppState>,
    req: web::Json<RegisterRequest>,
) -> Result<HttpResponse, ApiError> {
    let username = validate_username(&req.username)?;
    validate_password(&req.password)?;

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
        .register_user_with_password_and_api_key(
            username,
            &password_hash,
            &api_key,
            created_at,
        )
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
    if req.password.is_empty() {
        return Err(ApiError::BadRequest("password is required".into()));
    }

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

    let created_at = now_secs();
    let token = match state
        .user_service
        .get_first_api_key_for_user(user_id)
        .map_err(ApiError::from)?
    {
        Some(key) => key,
        None => {
            let key = create_api_key();
            state
                .user_service
                .create_api_key_for_user_id(user_id, &key, created_at)
                .map_err(ApiError::from)?;
            key
        }
    };

    let role = if username.eq_ignore_ascii_case("admin") {
        "admin"
    } else {
        "user"
    };

    Ok(HttpResponse::Ok().json(LoginResponse {
        token,
        user: LoginUserDto {
            username: username.to_string(),
            role: role.to_string(),
        },
    }))
}
