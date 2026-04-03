use actix_web::{web, HttpRequest, HttpResponse};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{auth, errors::ApiError, AppState};

fn auth_user_id(req: &HttpRequest, state: &web::Data<AppState>) -> Result<i64, ApiError> {
    let api_key = auth::extract_bearer_token(req)
        .ok_or_else(|| ApiError::Unauthorized("Invalid or missing API key".into()))?;
    let (_token_id, user_id) = state.auth_service.get_api_key_scope(api_key)?;
    Ok(user_id)
}

#[derive(Serialize)]
pub struct ApiKeyListResponse {
    pub data: Vec<crate::services::repository::ApiKeySummary>,
}

pub async fn list_my_api_keys(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let data = state.user_service.list_my_api_keys(user_id)?;
    Ok(HttpResponse::Ok().json(ApiKeyListResponse { data }))
}

#[derive(Serialize)]
pub struct CreateMyApiKeyResponse {
    pub id: i64,
    pub api_key: String,
    pub created_at: u64,
}

pub async fn create_my_api_key(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (id, api_key, created_at) = state
        .user_service
        .create_my_api_key(user_id, created_at)?;
    Ok(HttpResponse::Created().json(CreateMyApiKeyResponse {
        id,
        api_key,
        created_at,
    }))
}

pub async fn revoke_my_api_key(
    req: HttpRequest,
    state: web::Data<AppState>,
    key_id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    state
        .user_service
        .revoke_my_api_key(user_id, key_id.into_inner())?;
    Ok(HttpResponse::Ok().finish())
}
