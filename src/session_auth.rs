use actix_web::HttpRequest;

use crate::{auth, errors::ApiError, AppState};

pub struct ConsoleSession {
    pub user_id: i64,
    pub api_key_id: Option<i64>,
}

/// Resolves either a console JWT (Bearer) or an `sk-or-v1-*` API key for `/api/v1/*` routes.
pub fn resolve_console_session(
    req: &HttpRequest,
    state: &AppState,
) -> Result<ConsoleSession, ApiError> {
    let bearer = auth::extract_bearer_token(req)
        .ok_or_else(|| ApiError::Unauthorized("Invalid or missing credentials".into()))?;
    if bearer.starts_with("sk-or-v1-") {
        let (tid, uid) = state.auth_service.get_api_key_scope(bearer)?;
        Ok(ConsoleSession {
            user_id: uid,
            api_key_id: Some(tid),
        })
    } else {
        let claims = crate::jwt_session::decode_session_jwt(&state.cfg.auth.jwt_secret, bearer)
            .map_err(|_| ApiError::Unauthorized("Invalid or expired session".into()))?;
        Ok(ConsoleSession {
            user_id: claims.user_id,
            api_key_id: None,
        })
    }
}

pub fn parse_x_team_id(req: &HttpRequest) -> Option<i64> {
    let h = req.headers().get("X-Team-Id")?;
    let s = h.to_str().ok()?.trim();
    if s.is_empty() {
        return None;
    }
    s.parse().ok()
}

/// Resolves [crate::db::AuditConsoleScope] from optional `X-Team-Id` header (membership checked).
pub fn audit_scope_for_request(
    req: &HttpRequest,
    state: &AppState,
    user_id: i64,
) -> Result<crate::db::AuditConsoleScope, ApiError> {
    match parse_x_team_id(req) {
        None => Ok(crate::db::AuditConsoleScope::Personal(user_id)),
        Some(tid) => {
            let conn = state
                .db
                .get()
                .map_err(|_| ApiError::InternalError("database pool error".into()))?;
            if !crate::db::user_is_team_member(&conn, tid, user_id)
                .map_err(|_| ApiError::InternalError("database error".into()))?
            {
                return Err(ApiError::Forbidden("not a member of this team".into()));
            }
            Ok(crate::db::AuditConsoleScope::Team(tid))
        }
    }
}

/// Optional team context from `X-Team-Id` with membership enforcement.
pub fn team_context_or_none(
    req: &HttpRequest,
    state: &AppState,
    user_id: i64,
) -> Result<Option<i64>, ApiError> {
    match parse_x_team_id(req) {
        None => Ok(None),
        Some(tid) => {
            let conn = state
                .db
                .get()
                .map_err(|_| ApiError::InternalError("database pool error".into()))?;
            if !crate::db::user_is_team_member(&conn, tid, user_id)
                .map_err(|_| ApiError::InternalError("database error".into()))?
            {
                return Err(ApiError::Forbidden("not a member of this team".into()));
            }
            Ok(Some(tid))
        }
    }
}
