use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::ApiKeyPatchDb;
use crate::money;
use crate::services::user::CreateMyApiKeyInput;
use crate::{byok, db, errors::ApiError, session_auth, AppState};

fn auth_user_id(req: &HttpRequest, state: &web::Data<AppState>) -> Result<i64, ApiError> {
    Ok(session_auth::resolve_console_session(req, state)?.user_id)
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
    let team_id = session_auth::team_context_or_none(&req, &state, user_id)?;
    let data = state.user_service.list_my_api_keys(user_id, team_id)?;
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
    #[serde(default)]
    pub default_byok_profile_id: Option<i64>,
    /// Max simultaneous chat completions; omit or `null` = unlimited.
    #[serde(default)]
    pub max_concurrent_requests: Option<i32>,
    /// Monthly platform spend cap as USD minor string (k=15); omit = unset.
    #[serde(default)]
    pub quota_monthly_spend_minor: Option<String>,
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
            default_byok_profile_id: None,
            max_concurrent_requests: None,
            quota_monthly_spend_minor: None,
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
    let team_id = session_auth::team_context_or_none(&req, &state, user_id)?;
    if let Some(tid) = team_id {
        let conn = state
            .db
            .get()
            .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
        if !db::is_team_admin_or_owner(&conn, tid, user_id)
            .map_err(|_| ApiError::InternalError("database error".into()))?
        {
            return Err(ApiError::Forbidden(
                "team owner or admin required to create team API keys".into(),
            ));
        }
    }
    let created_at = now_secs();
    let b: CreateMyApiKeyBody =
        if bytes.is_empty() || bytes.as_ref().iter().all(|c| c.is_ascii_whitespace()) {
            CreateMyApiKeyBody::default()
        } else {
            serde_json::from_slice(&bytes)
                .map_err(|e| ApiError::BadRequest(format!("invalid JSON: {e}")))?
        };
    if let Some(pid) = b.default_byok_profile_id {
        if pid <= 0 {
            return Err(ApiError::BadRequest(
                "default_byok_profile_id must be positive".into(),
            ));
        }
        let conn = state
            .db
            .get()
            .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
        let ok = byok::profile_bindable_for_gateway_key(&conn, pid, user_id, team_id)
            .map_err(|e| ApiError::InternalError(format!("database error: {e}")))?;
        if !ok {
            return Err(ApiError::BadRequest(
                "BYOK profile not found or not in this key's scope".into(),
            ));
        }
    }
    if let Some(n) = b.max_concurrent_requests {
        if !(0..=65_535).contains(&n) {
            return Err(ApiError::BadRequest(
                "max_concurrent_requests must be between 0 and 65535".into(),
            ));
        }
    }
    let quota_monthly_spend_minor: Option<i128> =
        match b.quota_monthly_spend_minor.as_ref().map(|s| s.trim()) {
            None | Some("") => None,
            Some(s) => {
                let v = money::minor_from_db(s).map_err(|_| {
                    ApiError::BadRequest(
                        "quota_monthly_spend_minor must be a valid integer string".into(),
                    )
                })?;
                if v <= 0 {
                    return Err(ApiError::BadRequest(
                        "quota_monthly_spend_minor must be positive when set".into(),
                    ));
                }
                Some(v)
            }
        };
    let input = CreateMyApiKeyInput {
        name: b.name,
        description: b.description,
        expires_at: b.expires_at,
        quota_monthly_tokens: b.quota_monthly_tokens,
        model_allowlist: b.model_allowlist,
        ip_allowlist: b.ip_allowlist,
        team_id,
        default_byok_profile_id: b.default_byok_profile_id,
        max_concurrent_requests: b.max_concurrent_requests,
        quota_monthly_spend_minor,
    };
    let (id, api_key, created_at) = state
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
    /// `null` clears default BYOK (use ModelGate `[upstream]`). Omit = no change.
    #[serde(default)]
    pub default_byok_profile_id: Option<Option<i64>>,
    #[serde(default)]
    pub max_concurrent_requests: Option<Option<i32>>,
    #[serde(default)]
    pub quota_monthly_spend_minor: Option<Option<String>>,
    #[serde(default)]
    pub session_affinity_enabled: Option<bool>,
    /// `null` clears pool; omit = unchanged; array replaces pool order (Round Robin order).
    #[serde(default)]
    pub upstream_pool: Option<Option<Vec<crate::session_upstream::UpstreamPoolEntry>>>,
}

fn patch_db_has_changes(p: &ApiKeyPatchDb) -> bool {
    p.name.is_some()
        || p.description.is_some()
        || p.disabled.is_some()
        || p.expires_at.is_some()
        || p.quota_monthly_tokens.is_some()
        || p.model_allowlist.is_some()
        || p.ip_allowlist.is_some()
        || p.default_byok_profile_id.is_some()
        || p.max_concurrent_requests.is_some()
        || p.quota_monthly_spend_minor.is_some()
        || p.session_affinity_enabled.is_some()
        || p.upstream_pool_json.is_some()
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
            return Err(ApiError::BadRequest("name must be 1–64 characters".into()));
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
        patch.model_allowlist =
            Some(m.map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "[]".to_string())));
    }
    if let Some(ip) = b.ip_allowlist {
        patch.ip_allowlist =
            Some(ip.map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "[]".to_string())));
    }
    if let Some(opt) = b.default_byok_profile_id {
        if let Some(pid) = opt {
            if pid <= 0 {
                return Err(ApiError::BadRequest(
                    "default_byok_profile_id must be positive".into(),
                ));
            }
            let conn = state
                .db
                .get()
                .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
            let row = crate::db::get_api_key_row_for_console(&conn, user_id, key_id)
                .map_err(|_| ApiError::NotFound("API key not found".into()))?;
            let ok =
                crate::byok::profile_bindable_for_gateway_key(&conn, pid, row.user_id, row.team_id)
                    .map_err(|e| ApiError::InternalError(format!("database error: {e}")))?;
            if !ok {
                return Err(ApiError::BadRequest(
                    "BYOK profile not found or not in this key's scope".into(),
                ));
            }
        }
        patch.default_byok_profile_id = Some(opt);
    }
    if let Some(m) = b.max_concurrent_requests {
        if let Some(n) = m {
            if !(0..=65_535).contains(&n) {
                return Err(ApiError::BadRequest(
                    "max_concurrent_requests must be between 0 and 65535".into(),
                ));
            }
        }
        patch.max_concurrent_requests = Some(m);
    }
    if let Some(s) = b.quota_monthly_spend_minor {
        patch.quota_monthly_spend_minor = Some(match s {
            None => None,
            Some(ref st) if st.trim().is_empty() => None,
            Some(st) => {
                let v = money::minor_from_db(st.trim()).map_err(|_| {
                    ApiError::BadRequest(
                        "quota_monthly_spend_minor must be a valid integer string".into(),
                    )
                })?;
                if v <= 0 {
                    return Err(ApiError::BadRequest(
                        "quota_monthly_spend_minor must be positive when set".into(),
                    ));
                }
                Some(v)
            }
        });
    }
    if b.session_affinity_enabled.is_some() || b.upstream_pool.is_some() {
        let conn = state
            .db
            .get()
            .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
        let row = crate::db::get_api_key_row_for_console(&conn, user_id, key_id)
            .map_err(|_| ApiError::NotFound("API key not found".into()))?;
        let current_pool: Vec<crate::session_upstream::UpstreamPoolEntry> = row
            .upstream_pool_json
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        let merged_pool = match &b.upstream_pool {
            None => current_pool,
            Some(None) => Vec::new(),
            Some(Some(v)) => v.clone(),
        };
        let merged_enabled = b
            .session_affinity_enabled
            .unwrap_or(row.session_affinity_enabled != 0);
        if !merged_pool.is_empty() {
            crate::session_upstream::validate_upstream_pool(&merged_pool)
                .map_err(|m| ApiError::BadRequest(format!("upstream_pool: {m}")))?;
            for e in &merged_pool {
                if let crate::session_upstream::UpstreamPoolEntry::Byok { byok_profile_id } = e {
                    let ok = crate::byok::profile_bindable_for_gateway_key(
                        &conn,
                        *byok_profile_id,
                        row.user_id,
                        row.team_id,
                    )
                    .map_err(|e| ApiError::InternalError(format!("database error: {e}")))?;
                    if !ok {
                        return Err(ApiError::BadRequest(
                            "BYOK profile in upstream_pool not in this key's scope".into(),
                        ));
                    }
                }
            }
        }
        if merged_enabled && merged_pool.is_empty() {
            return Err(ApiError::BadRequest(
                "session_affinity_enabled requires a non-empty upstream_pool".into(),
            ));
        }
        if let Some(en) = b.session_affinity_enabled {
            patch.session_affinity_enabled = Some(en);
        }
        if b.upstream_pool.is_some() {
            patch.upstream_pool_json = Some(if merged_pool.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&merged_pool).map_err(|e| {
                    ApiError::InternalError(format!("serialize upstream_pool: {e}"))
                })?)
            });
        }
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
        let _ =
            crate::db::insert_api_key_audit(&conn, user_id, kid, "revoke", now_secs() as i64, None);
    }
    Ok(HttpResponse::Ok().finish())
}
