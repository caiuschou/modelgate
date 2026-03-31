use actix_web::{web, HttpResponse};
use rand::Rng;
use rusqlite::ErrorCode;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::error;

use crate::{db, errors::ApiError, AppState};

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
    let random_part: String = (0..32).map(|_| format!("{:x}", rng.gen::<u8>() % 16)).collect();
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

    let mut conn = db::lock_db(&state.db);
    let tx = conn.transaction().map_err(|err| {
        error!(error = %err, "failed to begin transaction");
        ApiError::InternalError("Failed to create user".into())
    })?;

    if let Err(err) = tx.execute(
        "INSERT INTO users (username, created_at) VALUES (?1, ?2)",
        rusqlite::params![username, created_at],
    ) {
        if let rusqlite::Error::SqliteFailure(err_code, _) = &err {
            if err_code.code == ErrorCode::ConstraintViolation {
                return Err(ApiError::Conflict("username already exists".into()));
            }
        }
        error!(error = %err, "failed to create user");
        return Err(ApiError::InternalError("Failed to create user".into()));
    }

    let user_id = tx.last_insert_rowid();
    if let Err(err) = tx.execute(
        "INSERT INTO api_keys (user_id, api_key, created_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![user_id, api_key, created_at],
    ) {
        error!(error = %err, "failed to create user api key");
        return Err(ApiError::InternalError("Failed to create user".into()));
    }

    tx.commit().map_err(|err| {
        error!(error = %err, "failed to commit transaction");
        ApiError::InternalError("Failed to create user".into())
    })?;

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

    let conn = db::lock_db(&state.db);
    let user_id = db::find_user_id(&conn, &username)
        .map_err(|_| ApiError::NotFound("user not found".into()))?;

    conn.execute(
        "INSERT INTO api_keys (user_id, api_key, created_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![user_id, api_key, created_at],
    )
    .map_err(|err| {
        error!(error = %err, "failed to create api key for user");
        ApiError::InternalError("Failed to create api key".into())
    })?;

    Ok(HttpResponse::Created().json(CreateUserResponse {
        username,
        api_key,
        created_at,
    }))
}
