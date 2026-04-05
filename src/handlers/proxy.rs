use actix_web::{http::StatusCode as ActixStatusCode, web, HttpRequest, HttpResponse};
use async_stream::stream;
use bytes::Bytes;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use reqwest::header as reqwest_header;
use serde_json::Value;
use tracing::{debug, error, info, warn};

use crate::{
    api_key_policy, auth, errors::ApiError, upstream, AppState,
};

static UPSTREAM_HEADERS: Lazy<reqwest::header::HeaderMap> = Lazy::new(|| {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest_header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );

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
    let request_id = crate::audit::generate_request_id();
    let app_id = extract_app_id(&req);
    let start = std::time::Instant::now();
    let api_key = auth::extract_bearer_token(&req)
        .ok_or_else(|| ApiError::Unauthorized("Invalid or missing API key".into()))?;
    if !api_key.starts_with("sk-or-v1-") {
        return Err(ApiError::Unauthorized(
            "Chat completions requires an sk-or-v1-* gateway API key".into(),
        ));
    }

    let is_stream = upstream::is_stream_request(&body);
    let model = parse_model_from_request(&body);

    let auth_row = state.auth_service.get_api_key_auth(api_key)?;
    let token_id = auth_row.id;
    let user_id = auth_row.user_id;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    state
        .user_service
        .ensure_monthly_quota(token_id, now)
        .map_err(ApiError::from)?;

    api_key_policy::check_model_allowlist(
        auth_row.model_allowlist.as_deref(),
        model.as_deref(),
    )
    .map_err(|m| ApiError::Forbidden(m.into()))?;

    if let Some(ip) = api_key_policy::client_ip(&req) {
        api_key_policy::check_ip_allowlist(auth_row.ip_allowlist.as_deref(), ip)
            .map_err(|m| ApiError::Forbidden(m.into()))?;
    } else if auth_row
        .ip_allowlist
        .as_ref()
        .map(|s| !s.is_empty())
        .unwrap_or(false)
    {
        return Err(ApiError::Forbidden(
            "cannot determine client IP for this API key policy".into(),
        ));
    }

    state
        .user_service
        .touch_api_key_last_used(token_id, now)
        .map_err(ApiError::from)?;
    debug!(
        %request_id,
        user_id,
        token_id,
        model = model.as_deref(),
        stream = is_stream,
        ?app_id,
        "chat completions proxy request accepted"
    );
    let upstream_url = upstream::build_chat_completions_url(&state.cfg.upstream.base_url);
    let request_body_path =
        crate::audit::save_body_to_file(&state.audit_config, &request_id, "request", &body).ok();

    let req_builder = state
        .http
        .post(upstream_url)
        .headers(UPSTREAM_HEADERS.clone())
        .header(
            reqwest_header::AUTHORIZATION,
            format!("Bearer {}", state.cfg.upstream.api_key),
        )
        .body(body.clone());

    let upstream_resp = match req_builder.send().await {
        Ok(resp) => resp,
        Err(e) => {
            error!(
                %request_id,
                user_id,
                token_id,
                model = model.as_deref(),
                stream = is_stream,
                ?app_id,
                error = %e,
                "upstream request failed"
            );
            send_audit_record(
                &state,
                crate::audit::AuditRecord {
                    request_id,
                    user_id: Some(user_id),
                    token_id: Some(token_id),
                    channel_id: None,
                    model: model.clone(),
                    request_type: Some("chat".to_string()),
                    request_body_path,
                    response_body_path: None,
                    status_code: Some(500),
                    error_message: Some("Upstream request failed".to_string()),
                    prompt_tokens: None,
                    completion_tokens: None,
                    total_tokens: None,
                    cost: None,
                    latency_ms: Some(start.elapsed().as_millis() as i64),
                    app_id: app_id.clone(),
                    finish_reason: None,
                    metadata: None,
                    created_at: crate::audit::now_unix_secs(),
                },
            )
            .await;
            return Err(ApiError::InternalError("Upstream request failed".into()));
        }
    };

    let status = ActixStatusCode::from_u16(upstream_resp.status().as_u16())
        .unwrap_or(ActixStatusCode::BAD_GATEWAY);
    let status_i64 = i64::from(status.as_u16());

    if (400..500).contains(&status_i64) {
        warn!(
            %request_id,
            user_id,
            token_id,
            model = model.as_deref(),
            stream = is_stream,
            upstream_status = status_i64,
            latency_ms = start.elapsed().as_millis() as i64,
            ?app_id,
            "upstream returned client error status"
        );
    }

    if is_stream {
        let stream_request_id = request_id.clone();
        send_audit_record(
            &state,
            crate::audit::AuditRecord {
                request_id,
                user_id: Some(user_id),
                token_id: Some(token_id),
                channel_id: None,
                model: model.clone(),
                request_type: Some("chat".to_string()),
                request_body_path,
                response_body_path: None,
                status_code: Some(status_i64),
                error_message: None,
                prompt_tokens: None,
                completion_tokens: None,
                total_tokens: None,
                cost: None,
                latency_ms: Some(start.elapsed().as_millis() as i64),
                app_id: app_id.clone(),
                finish_reason: None,
                metadata: Some(serde_json::json!({ "stream": true })),
                created_at: crate::audit::now_unix_secs(),
            },
        )
        .await;
        info!(
            %stream_request_id,
            user_id,
            token_id,
            model = model.as_deref(),
            stream = true,
            upstream_status = status_i64,
            latency_ms = start.elapsed().as_millis() as i64,
            ?app_id,
            "chat completion proxied"
        );
        let st = state.clone();
        let status_ok = (200..300).contains(&status_i64);
        let stream = stream! {
            let mut upstream_stream = upstream_resp.bytes_stream();
            let mut buf: Vec<u8> = Vec::new();
            let mut last_usage: Option<i64> = None;
            while let Some(item) = upstream_stream.next().await {
                match item {
                    Ok(chunk) => {
                        feed_sse_usage_lines(&mut buf, chunk.as_ref(), &mut last_usage);
                        yield Ok::<Bytes, actix_web::Error>(chunk);
                    }
                    Err(e) => {
                        error!(
                            %stream_request_id,
                            error = %e,
                            "upstream stream read failed"
                        );
                        yield Err(actix_web::error::ErrorBadGateway(
                            "upstream stream read failed",
                        ));
                        return;
                    }
                }
            }
            flush_sse_usage_tail(&mut buf, &mut last_usage);
            if status_ok {
                if let Some(total) = last_usage {
                    let _ = st.user_service.increment_quota_tokens(token_id, total);
                }
            }
        };

        Ok(HttpResponse::build(status)
            .content_type("text/event-stream")
            .streaming(stream))
    } else {
        let bytes = upstream_resp.bytes().await.map_err(|e| {
            error!(
                %request_id,
                user_id,
                token_id,
                model = model.as_deref(),
                upstream_status = status_i64,
                ?app_id,
                error = %e,
                "upstream response read failed"
            );
            ApiError::InternalError("Failed to read upstream response".into())
        })?;
        let response_body_path =
            crate::audit::save_body_to_file(&state.audit_config, &request_id, "response", &bytes)
                .ok();
        let usage = parse_usage_cost_and_finish(&bytes);
        let log_request_id = request_id.clone();
        send_audit_record(
            &state,
            crate::audit::AuditRecord {
                request_id,
                user_id: Some(user_id),
                token_id: Some(token_id),
                channel_id: None,
                model: model.clone(),
                request_type: Some("chat".to_string()),
                request_body_path,
                response_body_path,
                status_code: Some(status_i64),
                error_message: if status_i64 >= 400 {
                    Some("Upstream returned error status".to_string())
                } else {
                    None
                },
                prompt_tokens: usage.0,
                completion_tokens: usage.1,
                total_tokens: usage.2,
                cost: usage.3,
                latency_ms: Some(start.elapsed().as_millis() as i64),
                app_id: app_id.clone(),
                finish_reason: usage.4,
                metadata: Some(serde_json::json!({ "stream": false })),
                created_at: crate::audit::now_unix_secs(),
            },
        )
        .await;
        info!(
            %log_request_id,
            user_id,
            token_id,
            model = model.as_deref(),
            stream = false,
            upstream_status = status_i64,
            latency_ms = start.elapsed().as_millis() as i64,
            ?app_id,
            prompt_tokens = usage.0,
            completion_tokens = usage.1,
            total_tokens = usage.2,
            "chat completion proxied"
        );

        if (200..300).contains(&status_i64) {
            if let Some(total) = usage.2 {
                let _ = state.user_service.increment_quota_tokens(token_id, total);
            }
        }

        Ok(HttpResponse::build(status)
            .content_type("application/json")
            .body(bytes))
    }
}

fn feed_sse_usage_lines(buf: &mut Vec<u8>, chunk: &[u8], last_usage: &mut Option<i64>) {
    buf.extend_from_slice(chunk);
    while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
        let line: Vec<u8> = buf.drain(..=pos).collect();
        parse_sse_data_line_for_usage(&line, last_usage);
    }
}

fn flush_sse_usage_tail(buf: &mut Vec<u8>, last_usage: &mut Option<i64>) {
    if buf.is_empty() {
        return;
    }
    let line = std::mem::take(buf);
    parse_sse_data_line_for_usage(&line, last_usage);
}

fn parse_sse_data_line_for_usage(line: &[u8], last_usage: &mut Option<i64>) {
    let s = String::from_utf8_lossy(line);
    let t = s.trim_end();
    let Some(rest) = t.strip_prefix("data: ") else {
        return;
    };
    let rest = rest.trim();
    if rest == "[DONE]" {
        return;
    }
    let Ok(v) = serde_json::from_str::<Value>(rest) else {
        return;
    };
    if let Some(u) = v
        .get("usage")
        .and_then(|u| u.get("total_tokens"))
        .and_then(|x| x.as_i64())
    {
        *last_usage = Some(u);
    }
}

fn parse_model_from_request(body: &[u8]) -> Option<String> {
    serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("model")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned)
        })
}

fn extract_app_id(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("x-app-id")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string)
}

type UsageTokensCostFinish = (
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<f64>,
    Option<String>,
);

/// OpenAI-style chat completion JSON: `usage` + `choices[0].finish_reason`.
fn parse_usage_cost_and_finish(body: &[u8]) -> UsageTokensCostFinish {
    let value = match serde_json::from_slice::<Value>(body) {
        Ok(v) => v,
        Err(_) => return (None, None, None, None, None),
    };

    let usage = value.get("usage");
    let prompt_tokens = usage
        .and_then(|u| u.get("prompt_tokens"))
        .and_then(|v| v.as_i64());
    let completion_tokens = usage
        .and_then(|u| u.get("completion_tokens"))
        .and_then(|v| v.as_i64());
    let total_tokens = usage
        .and_then(|u| u.get("total_tokens"))
        .and_then(|v| v.as_i64());
    let cost = value.get("cost").and_then(|v| v.as_f64());
    let finish_reason = value
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|ch| ch.get("finish_reason"))
        .and_then(|v| v.as_str())
        .map(std::string::ToString::to_string);

    (
        prompt_tokens,
        completion_tokens,
        total_tokens,
        cost,
        finish_reason,
    )
}

async fn send_audit_record(state: &web::Data<AppState>, record: crate::audit::AuditRecord) {
    if let Err(err) = state
        .audit_sender
        .send(crate::audit::AuditMessage::Record(record))
        .await
    {
        error!(error = %err, "failed to enqueue audit record");
    }
}
