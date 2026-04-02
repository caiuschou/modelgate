use actix_web::{web, HttpResponse};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{errors::ApiError, AppState};

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
}

#[derive(Serialize)]
pub struct CreateUserResponse {
    pub username: String,
    pub api_key: String,
    pub created_at: u64,
}

fn create_api_key() -> String {
    let mut rng = rand::thread_rng();
    let random_part: String = (0..32)
        .map(|_| format!("{:x}", rng.gen::<u8>() % 16))
        .collect();
    format!("sk-or-v1-{}", random_part)
}

pub async fn create_user(
    state: web::Data<AppState>,
    req: web::Json<CreateUserRequest>,
) -> Result<HttpResponse, ApiError> {
    let username = req.username.trim();
    if username.is_empty() {
        return Err(ApiError::BadRequest("username is required".into()));
    }

    let api_key = create_api_key();
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    state
        .user_service
        .create_user_with_api_key(username, &api_key, created_at)?;

    Ok(HttpResponse::Created().json(CreateUserResponse {
        username: username.to_string(),
        api_key,
        created_at,
    }))
}

pub async fn create_user_api_key(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let username = path.into_inner();
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let api_key = create_api_key();

    state
        .user_service
        .create_api_key_for_user(&username, &api_key, created_at)?;

    Ok(HttpResponse::Created().json(CreateUserResponse {
        username,
        api_key,
        created_at,
    }))
}
