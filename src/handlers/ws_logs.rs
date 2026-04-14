use actix_web::error::Error as ActixError;
use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::Message;
use futures_util::StreamExt as _;
use serde::Deserialize;

use crate::audit_notify::should_deliver_to_ws;
use crate::auth;
use crate::errors::ApiError;
use crate::session_auth;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct WsLogsQuery {
    pub access_token: Option<String>,
    pub team_id: Option<i64>,
}

fn bearer_token(req: &HttpRequest, q: &WsLogsQuery) -> Result<String, ApiError> {
    if let Some(t) = auth::extract_bearer_token(req) {
        return Ok(t.to_string());
    }
    let t = q
        .access_token
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    t.map(|s| s.to_string())
        .ok_or_else(|| ApiError::Unauthorized("Missing credentials".into()))
}

/// Browser WebSockets cannot set `Authorization`; pass JWT (or gateway API key) as `access_token`.
pub async fn audit_logs_ws(
    req: HttpRequest,
    body: web::Payload,
    state: web::Data<AppState>,
    query: web::Query<WsLogsQuery>,
) -> Result<HttpResponse, ActixError> {
    let token = bearer_token(&req, &query).map_err(ActixError::from)?;
    let session = resolve_ws_session(&state, &token).map_err(ActixError::from)?;
    let user_id = session.user_id;

    let ws_team = match query.team_id {
        None => None,
        Some(tid) => {
            let conn = state
                .db
                .get()
                .map_err(|e| ApiError::InternalError(e.to_string()))?;
            if !crate::db::user_is_team_member(&conn, tid, user_id)
                .map_err(|e| ApiError::InternalError(e.to_string()))?
            {
                return Err(ApiError::Forbidden("not a member of this team".into()).into());
            }
            Some(tid)
        }
    };

    let hub = state.audit_notify.clone();
    let mut rx = hub.subscribe();

    let (res, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    actix_web::rt::spawn(async move {
        loop {
            tokio::select! {
                ws_m = msg_stream.next() => {
                    match ws_m {
                        Some(Ok(Message::Ping(bytes))) => {
                            if session.pong(&bytes).await.is_err() {
                                break;
                            }
                        }
                        Some(Ok(Message::Pong(_))) => {}
                        Some(Ok(Message::Close(_))) | None => break,
                        Some(Err(_)) => break,
                        Some(Ok(_)) => {}
                    }
                }
                recv = rx.recv() => {
                    match recv {
                        Ok(payload) => {
                            if should_deliver_to_ws(&payload, user_id, ws_team) {
                                let Ok(txt) = serde_json::to_string(&payload) else { continue };
                                if session.text(txt).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                        Err(_) => break,
                    }
                }
            }
        }
        let _ = session.close(None).await;
    });

    Ok(res)
}

fn resolve_ws_session(
    state: &AppState,
    token: &str,
) -> Result<session_auth::ConsoleSession, ApiError> {
    if token.starts_with("sk-or-v1-") {
        let (tid, uid) = state.auth_service.get_api_key_scope(token)?;
        Ok(session_auth::ConsoleSession {
            user_id: uid,
            api_key_id: Some(tid),
        })
    } else {
        let claims = crate::jwt_session::decode_session_jwt(&state.cfg.auth.jwt_secret, token)
            .map_err(|_| ApiError::Unauthorized("Invalid or expired session".into()))?;
        Ok(session_auth::ConsoleSession {
            user_id: claims.user_id,
            api_key_id: None,
        })
    }
}
