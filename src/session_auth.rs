use actix_web::HttpRequest;

use crate::{auth, errors::ApiError, AppState};

pub struct ConsoleSession {
    pub user_id: i64,
    pub api_key_id: Option<i64>,
}

/// Resolves either a console JWT (Bearer) or an `sk-or-v1-*` API key for `/api/v1/*` routes.
pub fn resolve_console_session(req: &HttpRequest, state: &AppState) -> Result<ConsoleSession, ApiError> {
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
