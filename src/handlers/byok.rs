use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::byok::{
    self, fetch_byok_profile_stored, insert_byok_profile, list_byok_profiles,
    update_byok_profile_stored, ByokProfileSummary,
};
use crate::db;
use crate::errors::ApiError;
use crate::session_auth;
use crate::AppState;

fn require_byok_master_key(cfg: &crate::config::AppConfig) -> Result<[u8; 32], ApiError> {
    cfg.byok
        .master_key_32()
        .map_err(ApiError::ServiceUnavailable)
}

fn auth_user_id(req: &HttpRequest, state: &web::Data<AppState>) -> Result<i64, ApiError> {
    Ok(session_auth::resolve_console_session(req, state)?.user_id)
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[derive(Serialize)]
pub struct ByokListResponse {
    pub data: Vec<ByokProfileSummary>,
}

pub async fn list_my_byok_profiles(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let _ = require_byok_master_key(&state.cfg)?;
    let user_id = auth_user_id(&req, &state)?;
    let team_id = session_auth::team_context_or_none(&req, &state, user_id)?;
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    let data = list_byok_profiles(&conn, user_id, team_id)
        .map_err(|e| ApiError::InternalError(format!("database error: {e}")))?;
    Ok(HttpResponse::Ok().json(ByokListResponse { data }))
}

#[derive(Deserialize)]
pub struct CreateByokBody {
    #[serde(default = "default_name")]
    pub name: String,
    pub base_url: String,
    pub api_key: String,
}

fn default_name() -> String {
    "BYOK profile".to_string()
}

#[derive(Serialize)]
pub struct CreateByokResponse {
    pub id: i64,
    pub created_at: i64,
}

pub async fn create_my_byok_profile(
    req: HttpRequest,
    state: web::Data<AppState>,
    bytes: web::Bytes,
) -> Result<HttpResponse, ApiError> {
    let master = require_byok_master_key(&state.cfg)?;
    let user_id = auth_user_id(&req, &state)?;
    let team_id = session_auth::team_context_or_none(&req, &state, user_id)?;
    if let Some(tid) = team_id {
        let conn = state
            .db
            .get()
            .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
        if !db::is_team_admin_or_owner(&conn, tid, user_id)
            .map_err(|e| ApiError::InternalError(format!("database error: {e}")))?
        {
            return Err(ApiError::Forbidden(
                "team owner or admin required to create team BYOK profiles".into(),
            ));
        }
    }

    let b: CreateByokBody = serde_json::from_slice(&bytes)
        .map_err(|e| ApiError::BadRequest(format!("invalid JSON: {e}")))?;
    let base_url = b.base_url.trim().to_string();
    if base_url.is_empty() {
        return Err(ApiError::BadRequest("base_url is required".into()));
    }
    let api_key = b.api_key.trim().to_string();
    if api_key.is_empty() {
        return Err(ApiError::BadRequest("api_key is required".into()));
    }

    let sealed = byok::seal_upstream_api_key(&master, api_key.as_bytes())
        .map_err(|_| ApiError::InternalError("failed to seal upstream API key".into()))?;
    let (nonce, ciphertext) = byok::split_sealed_blob(&sealed)
        .map_err(|_| ApiError::InternalError("invalid sealed blob".into()))?;

    let now = now_secs();
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    let id = if team_id.is_none() {
        insert_byok_profile(
            &conn,
            Some(user_id),
            None,
            &b.name,
            &base_url,
            nonce,
            ciphertext,
            now,
        )
    } else {
        insert_byok_profile(
            &conn, None, team_id, &b.name, &base_url, nonce, ciphertext, now,
        )
    }
    .map_err(|e| ApiError::InternalError(format!("database error: {e}")))?;

    Ok(HttpResponse::Ok().json(CreateByokResponse {
        id,
        created_at: now,
    }))
}

pub async fn get_my_byok_profile(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let _ = require_byok_master_key(&state.cfg)?;
    let user_id = auth_user_id(&req, &state)?;
    let team_id = session_auth::team_context_or_none(&req, &state, user_id)?;
    let id = path.into_inner();
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    let row = byok::get_byok_profile_detail(&conn, id, user_id, team_id)
        .map_err(|e| ApiError::InternalError(format!("database error: {e}")))?;
    let Some(s) = row else {
        return Err(ApiError::NotFound("BYOK profile not found".into()));
    };
    Ok(HttpResponse::Ok().json(s))
}

#[derive(Deserialize, Default)]
pub struct PatchByokBody {
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
}

pub async fn patch_my_byok_profile(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    bytes: web::Bytes,
) -> Result<HttpResponse, ApiError> {
    let master = require_byok_master_key(&state.cfg)?;
    let user_id = auth_user_id(&req, &state)?;
    let team_id = session_auth::team_context_or_none(&req, &state, user_id)?;
    if let Some(tid) = team_id {
        let conn = state
            .db
            .get()
            .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
        if !db::is_team_admin_or_owner(&conn, tid, user_id)
            .map_err(|e| ApiError::InternalError(format!("database error: {e}")))?
        {
            return Err(ApiError::Forbidden(
                "team owner or admin required to update team BYOK profiles".into(),
            ));
        }
    }

    let id = path.into_inner();
    let body: PatchByokBody = if bytes.is_empty() {
        PatchByokBody::default()
    } else {
        serde_json::from_slice(&bytes)
            .map_err(|e| ApiError::BadRequest(format!("invalid JSON: {e}")))?
    };

    if body.name.is_none() && body.base_url.is_none() && body.api_key.is_none() {
        return Err(ApiError::BadRequest(
            "at least one of name, base_url, api_key is required".into(),
        ));
    }

    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    let Some(mut stored) = fetch_byok_profile_stored(&conn, id, user_id, team_id)
        .map_err(|e| ApiError::InternalError(format!("database error: {e}")))?
    else {
        return Err(ApiError::NotFound("BYOK profile not found".into()));
    };

    if let Some(n) = body.name {
        stored.name = n;
    }
    if let Some(b) = body.base_url {
        let t = b.trim().to_string();
        if t.is_empty() {
            return Err(ApiError::BadRequest("base_url cannot be empty".into()));
        }
        stored.base_url = t;
    }
    if let Some(k) = body.api_key {
        let t = k.trim().to_string();
        if t.is_empty() {
            return Err(ApiError::BadRequest("api_key cannot be empty".into()));
        }
        let sealed = byok::seal_upstream_api_key(&master, t.as_bytes())
            .map_err(|_| ApiError::InternalError("failed to seal upstream API key".into()))?;
        let (nonce, ciphertext) = byok::split_sealed_blob(&sealed)
            .map_err(|_| ApiError::InternalError("invalid sealed blob".into()))?;
        stored.api_key_nonce = nonce.to_vec();
        stored.api_key_ciphertext = ciphertext.to_vec();
    }

    let now = now_secs();
    let n = update_byok_profile_stored(&conn, id, user_id, team_id, &stored, now)
        .map_err(|e| ApiError::InternalError(format!("database error: {e}")))?;
    if n == 0 {
        return Err(ApiError::NotFound("BYOK profile not found".into()));
    }
    Ok(HttpResponse::Ok().json(serde_json::json!({ "ok": true, "updated_at": now })))
}

pub async fn revoke_my_byok_profile(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let _ = require_byok_master_key(&state.cfg)?;
    let user_id = auth_user_id(&req, &state)?;
    let team_id = session_auth::team_context_or_none(&req, &state, user_id)?;
    if let Some(tid) = team_id {
        let conn = state
            .db
            .get()
            .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
        if !db::is_team_admin_or_owner(&conn, tid, user_id)
            .map_err(|e| ApiError::InternalError(format!("database error: {e}")))?
        {
            return Err(ApiError::Forbidden(
                "team owner or admin required to revoke team BYOK profiles".into(),
            ));
        }
    }
    let id = path.into_inner();
    let now = now_secs();
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    let n = byok::revoke_byok_profile(&conn, id, user_id, team_id, now)
        .map_err(|e| ApiError::InternalError(format!("database error: {e}")))?;
    if n == 0 {
        return Err(ApiError::NotFound("BYOK profile not found".into()));
    }
    byok::clear_default_byok_refs_for_profile(&conn, id)
        .map_err(|e| ApiError::InternalError(format!("database error: {e}")))?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "ok": true })))
}
