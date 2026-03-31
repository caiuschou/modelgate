use actix_web::{http::StatusCode as ActixStatusCode, web, HttpRequest, HttpResponse};
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use reqwest::header as reqwest_header;
use tracing::error;

use crate::{auth, db, errors::ApiError, upstream, AppState};

static UPSTREAM_HEADERS: Lazy<reqwest::header::HeaderMap> = Lazy::new(|| {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest_header::CONTENT_TYPE, "application/json".parse().unwrap());
    
    if let Ok(org) = std::env::var("OPENAI_ORGANIZATION") {
        if let Ok(header) = org.parse() {
            headers.insert("openai-organization", header);
        }
    }
    if let Ok(project) = std::env::var("OPENAI_PROJECT") {
        if let Ok(header) = project.parse() {
            headers.insert("openai-project", header);
        }
    }
    headers
});

pub async fn chat_completions(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Bytes,
) -> Result<HttpResponse, ApiError> {
    let api_key = auth::extract_bearer_token(&req)
        .ok_or_else(|| ApiError::Unauthorized("Invalid or missing API key".into()))?;

    {
        let db = db::lock_db(&state.db);
        if !auth::validate_api_key(&db, api_key) {
            return Err(ApiError::Unauthorized("Invalid or missing API key".into()));
        }
    }

    let is_stream = upstream::is_stream_request(&body);
    let upstream_url = upstream::build_chat_completions_url(&state.cfg.upstream.base_url);

    let req_builder = state
        .http
        .post(upstream_url)
        .headers(UPSTREAM_HEADERS.clone())
        .header(
            reqwest_header::AUTHORIZATION,
            format!("Bearer {}", state.cfg.upstream.api_key),
        )
        .body(body);

    let upstream_resp = req_builder.send().await.map_err(|e| {
        error!(error = %e, "upstream request failed");
        ApiError::InternalError("Upstream request failed".into())
    })?;

    let status = ActixStatusCode::from_u16(upstream_resp.status().as_u16())
        .unwrap_or(ActixStatusCode::BAD_GATEWAY);

    if is_stream {
        let stream = upstream_resp.bytes_stream().map(|chunk| {
            chunk.map_err(|e| {
                error!(error = %e, "upstream stream read failed");
                actix_web::error::ErrorBadGateway("upstream stream read failed")
            })
        });

        Ok(HttpResponse::build(status)
            .content_type("text/event-stream")
            .streaming(stream))
    } else {
        let bytes = upstream_resp.bytes().await.map_err(|e| {
            error!(error = %e, "upstream response read failed");
            ApiError::InternalError("Failed to read upstream response".into())
        })?;

        Ok(HttpResponse::build(status)
            .content_type("application/json")
            .body(bytes))
    }
}
