use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::ApiKeyPatchDb;
use crate::services::user::CreateMyApiKeyInput;
use crate::{auth, errors::ApiError, AppState};

fn auth_user_id(req: &HttpRequest, state: &web::Data<AppState>) -> Result<i64, ApiError> {
    let api_key = auth::extract_bearer_token(req)
        .ok_or_else(|| ApiError::Unauthorized("Invalid or missing API key".into()))?;
    let (_token_id, user_id) = state.auth_service.get_api_key_scope(api_key)?;
    Ok(user_id)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
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

fn default_create_key_name() -> String {
    "未命名密钥".to_string()
}

#[derive(Deserialize)]
pub struct CreateMyApiKeyBody {
    #[serde(default = "default_create_key_name")]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub expires_at: Option<u64>,
    #[serde(default)]
    pub quota_monthly_tokens: Option<i64>,
    #[serde(default)]
    pub model_allowlist: Option<Vec<String>>,
    #[serde(default)]
    pub ip_allowlist: Option<Vec<String>>,
}

impl Default for CreateMyApiKeyBody {
    fn default() -> Self {
        Self {
            name: default_create_key_name(),
            description: None,
            expires_at: None,
            quota_monthly_tokens: None,
            model_allowlist: None,
            ip_allowlist: None,
        }
    }
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
    bytes: web::Bytes,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let created_at = now_secs();
    let b: CreateMyApiKeyBody = if bytes.is_empty()
        || bytes
            .as_ref()
            .iter()
            .all(|c| c.is_ascii_whitespace())
    {
        CreateMyApiKeyBody::default()
    } else {
        serde_json::from_slice(&bytes)
            .map_err(|e| ApiError::BadRequest(format!("invalid JSON: {e}")))?
    };
    let input = CreateMyApiKeyInput {
        name: b.name,
        description: b.description,
        expires_at: b.expires_at,
        quota_monthly_tokens: b.quota_monthly_tokens,
        model_allowlist: b.model_allowlist,
        ip_allowlist: b.ip_allowlist,
    };
    let (id, api_key, created_at) =
        state
            .user_service
            .create_my_api_key(user_id, created_at, input)?;
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    crate::db::insert_api_key_audit(&conn, user_id, id, "create", created_at as i64, None)
        .map_err(|_| ApiError::InternalError("failed to write key audit".into()))?;
    Ok(HttpResponse::Created().json(CreateMyApiKeyResponse {
        id,
        api_key,
        created_at,
    }))
}

pub async fn get_my_api_key(
    req: HttpRequest,
    state: web::Data<AppState>,
    key_id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let data = state
        .user_service
        .get_my_api_key(user_id, key_id.into_inner())?;
    Ok(HttpResponse::Ok().json(data))
}

#[derive(Deserialize)]
pub struct PatchMyApiKeyBody {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub disabled: Option<bool>,
    #[serde(default)]
    pub expires_at: Option<Option<u64>>,
    #[serde(default)]
    pub quota_monthly_tokens: Option<Option<i64>>,
    #[serde(default)]
    pub model_allowlist: Option<Option<Vec<String>>>,
    #[serde(default)]
    pub ip_allowlist: Option<Option<Vec<String>>>,
}

fn patch_db_has_changes(p: &ApiKeyPatchDb) -> bool {
    p.name.is_some()
        || p.description.is_some()
        || p.disabled.is_some()
        || p.expires_at.is_some()
        || p.quota_monthly_tokens.is_some()
        || p.model_allowlist.is_some()
        || p.ip_allowlist.is_some()
}

pub async fn patch_my_api_key(
    req: HttpRequest,
    state: web::Data<AppState>,
    key_id: web::Path<i64>,
    body: web::Json<PatchMyApiKeyBody>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let key_id = key_id.into_inner();
    let b = body.into_inner();
    let mut patch = ApiKeyPatchDb::default();
    if let Some(ref n) = b.name {
        let t = n.trim();
        if t.is_empty() || t.len() > 64 {
            return Err(ApiError::BadRequest(
                "name must be 1–64 characters".into(),
            ));
        }
        patch.name = Some(t.to_string());
    }
    if let Some(ref d) = b.description {
        if d.len() > 512 {
            return Err(ApiError::BadRequest(
                "description must be at most 512 characters".into(),
            ));
        }
        patch.description = Some(d.clone());
    }
    if let Some(d) = b.disabled {
        patch.disabled = Some(d);
    }
    if let Some(e) = b.expires_at {
        patch.expires_at = Some(e.map(|u| u as i64));
    }
    if let Some(q) = b.quota_monthly_tokens {
        patch.quota_monthly_tokens = Some(q);
    }
    if let Some(m) = b.model_allowlist {
        patch.model_allowlist = Some(m.map(|v| {
            serde_json::to_string(&v).unwrap_or_else(|_| "[]".to_string())
        }));
    }
    if let Some(ip) = b.ip_allowlist {
        patch.ip_allowlist = Some(ip.map(|v| {
            serde_json::to_string(&v).unwrap_or_else(|_| "[]".to_string())
        }));
    }
    if !patch_db_has_changes(&patch) {
        return Err(ApiError::BadRequest("no fields to update".into()));
    }
    state
        .user_service
        .update_my_api_key(user_id, key_id, patch)?;
    if let Ok(conn) = state.db.get() {
        let _ = crate::db::insert_api_key_audit(
            &conn,
            user_id,
            key_id,
            "update",
            now_secs() as i64,
            None,
        );
    }
    Ok(HttpResponse::Ok().finish())
}

pub async fn revoke_my_api_key(
    req: HttpRequest,
    state: web::Data<AppState>,
    key_id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let user_id = auth_user_id(&req, &state)?;
    let kid = key_id.into_inner();
    state.user_service.revoke_my_api_key(user_id, kid)?;
    if let Ok(conn) = state.db.get() {
        let _ = crate::db::insert_api_key_audit(
            &conn,
            user_id,
            kid,
            "revoke",
            now_secs() as i64,
            None,
        );
    }
    Ok(HttpResponse::Ok().finish())
}
