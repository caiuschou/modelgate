use actix_web::{http::StatusCode as ActixStatusCode, web, HttpRequest, HttpResponse};
use async_stream::stream;
use bytes::Bytes;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use reqwest::header as reqwest_header;
use serde_json::Value;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, info, warn};

use crate::{
    api_key_policy, auth, byok, byok::ByokResolveError, db::ApiKeyAuthRow, errors::ApiError,
    upstream, AppState,
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
    let key_team_id = auth_row.team_id;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    state
        .user_service
        .ensure_monthly_quota(token_id, now)
        .map_err(ApiError::from)?;

    api_key_policy::check_model_allowlist(auth_row.model_allowlist.as_deref(), model.as_deref())
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

    let resolved = resolve_chat_upstream(&req, &state, &auth_row)?;
    debug!(
        %request_id,
        user_id,
        token_id,
        model = model.as_deref(),
        stream = is_stream,
        ?app_id,
        is_byok = resolved.is_byok,
        "chat completions proxy request accepted"
    );
    let upstream_url = upstream::build_chat_completions_url(&resolved.base_url);
    let request_body_path =
        crate::audit::save_body_to_file(&state.audit_config, &request_id, "request", &body).ok();

    let req_builder = state
        .http
        .post(upstream_url)
        .headers(UPSTREAM_HEADERS.clone())
        .header(
            reqwest_header::AUTHORIZATION,
            format!("Bearer {}", resolved.api_key),
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
                    metadata: Some(chat_audit_metadata(
                        is_stream,
                        resolved.is_byok,
                        resolved.byok_profile_id,
                        None,
                    )),
                    created_at: crate::audit::now_unix_secs(),
                    team_id: key_team_id,
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
                metadata: Some(chat_audit_metadata(
                    true,
                    resolved.is_byok,
                    resolved.byok_profile_id,
                    None,
                )),
                created_at: crate::audit::now_unix_secs(),
                team_id: key_team_id,
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
        let stream_is_byok = resolved.is_byok;
        let stream_byok_id = resolved.byok_profile_id;
        let stream = stream! {
            let mut file_pair = match crate::audit::create_stream_response_body_file(
                &st.audit_config,
                &stream_request_id,
            )
            .await
            {
                Ok(pair) => Some(pair),
                Err(e) => {
                    error!(
                        %stream_request_id,
                        error = %e,
                        "failed to create stream response audit file"
                    );
                    None
                }
            };
            let mut usage_state: UsageTokensCostFinish =
                (None, None, None, None, None);
            let mut upstream_stream = upstream_resp.bytes_stream();
            let mut buf: Vec<u8> = Vec::new();
            while let Some(item) = upstream_stream.next().await {
                match item {
                    Ok(chunk) => {
                        feed_sse_usage_lines(&mut buf, chunk.as_ref(), &mut usage_state);
                        if let Some((_path, ref mut f)) = file_pair.as_mut() {
                            if let Err(e) = f.write_all(chunk.as_ref()).await {
                                error!(
                                    %stream_request_id,
                                    error = %e,
                                    "failed to append stream chunk to audit file"
                                );
                            }
                        }
                        yield Ok::<Bytes, actix_web::Error>(chunk);
                    }
                    Err(e) => {
                        error!(
                            %stream_request_id,
                            error = %e,
                            "upstream stream read failed"
                        );
                        if let Some((path, mut f)) = file_pair.take() {
                            let _ = f.flush().await;
                            let _ = f.shutdown().await;
                            enqueue_stream_audit_completion(
                                &st,
                                StreamAuditCompletionJob {
                                    request_id: stream_request_id.clone(),
                                    response_body_path: path,
                                    usage: usage_state.clone(),
                                    latency_ms: start.elapsed().as_millis() as i64,
                                    stream_completed: false,
                                    stream_aborted: true,
                                    error_message: Some("Upstream stream read failed".into()),
                                    is_byok: stream_is_byok,
                                    byok_profile_id: stream_byok_id,
                                },
                            )
                            .await;
                        }
                        yield Err(actix_web::error::ErrorBadGateway(
                            "upstream stream read failed",
                        ));
                        return;
                    }
                }
            }
            flush_sse_usage_tail(&mut buf, &mut usage_state);
            if let Some((path, mut f)) = file_pair.take() {
                let _ = f.flush().await;
                let _ = f.shutdown().await;
                enqueue_stream_audit_completion(
                    &st,
                    StreamAuditCompletionJob {
                        request_id: stream_request_id.clone(),
                        response_body_path: path,
                        usage: usage_state.clone(),
                        latency_ms: start.elapsed().as_millis() as i64,
                        stream_completed: true,
                        stream_aborted: false,
                        error_message: None,
                        is_byok: stream_is_byok,
                        byok_profile_id: stream_byok_id,
                    },
                )
                .await;
            }
            if status_ok {
                if let Some(total) = usage_state.2 {
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
                metadata: Some(chat_audit_metadata(
                    false,
                    resolved.is_byok,
                    resolved.byok_profile_id,
                    None,
                )),
                created_at: crate::audit::now_unix_secs(),
                team_id: key_team_id,
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

fn feed_sse_usage_lines(buf: &mut Vec<u8>, chunk: &[u8], usage: &mut UsageTokensCostFinish) {
    buf.extend_from_slice(chunk);
    while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
        let line: Vec<u8> = buf.drain(..=pos).collect();
        parse_sse_data_line_merge_usage(&line, usage);
    }
}

fn flush_sse_usage_tail(buf: &mut Vec<u8>, usage: &mut UsageTokensCostFinish) {
    if buf.is_empty() {
        return;
    }
    let line = std::mem::take(buf);
    parse_sse_data_line_merge_usage(&line, usage);
}

fn parse_sse_data_line_merge_usage(line: &[u8], usage: &mut UsageTokensCostFinish) {
    let s = String::from_utf8_lossy(line);
    let t = s.trim_end();
    let Some(rest) = t.strip_prefix("data: ") else {
        return;
    };
    let rest = rest.trim();
    if rest == "[DONE]" {
        return;
    }
    merge_usage_tokens(usage, rest.as_bytes());
}

fn merge_usage_tokens(usage: &mut UsageTokensCostFinish, chunk: &[u8]) {
    let (p, c, t, co, fr) = parse_usage_cost_and_finish(chunk);
    if p.is_some() {
        usage.0 = p;
    }
    if c.is_some() {
        usage.1 = c;
    }
    if t.is_some() {
        usage.2 = t;
    }
    if co.is_some() {
        usage.3 = co;
    }
    if fr.is_some() {
        usage.4 = fr;
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

struct StreamAuditCompletionJob {
    request_id: String,
    response_body_path: String,
    usage: UsageTokensCostFinish,
    latency_ms: i64,
    stream_completed: bool,
    stream_aborted: bool,
    error_message: Option<String>,
    is_byok: bool,
    byok_profile_id: Option<i64>,
}

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

async fn enqueue_stream_audit_completion(
    state: &web::Data<AppState>,
    job: StreamAuditCompletionJob,
) {
    let StreamAuditCompletionJob {
        request_id,
        response_body_path,
        usage,
        latency_ms,
        stream_completed,
        stream_aborted,
        error_message,
        is_byok,
        byok_profile_id,
    } = job;
    let mut stream_extra = serde_json::Map::new();
    stream_extra.insert("stream_completed".into(), stream_completed.into());
    stream_extra.insert("stream_aborted".into(), stream_aborted.into());
    stream_extra.insert("response_body_format".into(), "text/event-stream".into());
    let metadata = chat_audit_metadata(true, is_byok, byok_profile_id, Some(stream_extra));
    let update = crate::audit::AuditStreamCompletionUpdate {
        request_id,
        response_body_path,
        prompt_tokens: usage.0,
        completion_tokens: usage.1,
        total_tokens: usage.2,
        cost: usage.3,
        finish_reason: usage.4,
        latency_ms,
        metadata,
        error_message,
    };
    if let Err(err) = state
        .audit_sender
        .send(crate::audit::AuditMessage::StreamCompletion(update))
        .await
    {
        error!(error = %err, "failed to enqueue stream audit completion");
    }
}

#[derive(Debug)]
struct ResolvedUpstream {
    base_url: String,
    api_key: String,
    is_byok: bool,
    byok_profile_id: Option<i64>,
}

fn parse_use_platform_upstream(req: &HttpRequest) -> bool {
    let Some(raw) = req.headers().get("x-mg-use-platform-upstream") else {
        return false;
    };
    let s = raw.to_str().unwrap_or("").trim().to_ascii_lowercase();
    matches!(s.as_str(), "1" | "true" | "yes")
}

fn parse_x_byok_profile_id(req: &HttpRequest) -> Result<Option<i64>, ApiError> {
    let Some(raw) = req.headers().get("x-mg-byok-id") else {
        return Ok(None);
    };
    let s = raw
        .to_str()
        .map_err(|_| ApiError::BadRequest("invalid X-MG-Byok-Id header".into()))?
        .trim();
    if s.is_empty() {
        return Ok(None);
    }
    let v = s
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("invalid X-MG-Byok-Id".into()))?;
    if v <= 0 {
        return Err(ApiError::BadRequest(
            "X-MG-Byok-Id must be a positive profile id".into(),
        ));
    }
    Ok(Some(v))
}

fn resolve_chat_upstream(
    req: &HttpRequest,
    state: &web::Data<AppState>,
    auth_row: &ApiKeyAuthRow,
) -> Result<ResolvedUpstream, ApiError> {
    let user_id = auth_row.user_id;
    let api_key_team_id = auth_row.team_id;

    if parse_use_platform_upstream(req) {
        return Ok(ResolvedUpstream {
            base_url: state.cfg.upstream.base_url.clone(),
            api_key: state.cfg.upstream.api_key.clone(),
            is_byok: false,
            byok_profile_id: None,
        });
    }

    let explicit = parse_x_byok_profile_id(req)?;
    let profile_id = explicit.or(auth_row.default_byok_profile_id);

    let Some(pid) = profile_id else {
        return Ok(ResolvedUpstream {
            base_url: state.cfg.upstream.base_url.clone(),
            api_key: state.cfg.upstream.api_key.clone(),
            is_byok: false,
            byok_profile_id: None,
        });
    };

    if pid <= 0 {
        return Err(ApiError::BadRequest("invalid default BYOK profile id".into()));
    }

    let master = state
        .cfg
        .byok
        .master_key_32()
        .map_err(ApiError::ServiceUnavailable)?;
    let conn = state
        .db
        .get()
        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
    match byok::resolve_byok_for_gateway(&conn, pid, user_id, api_key_team_id, &master) {
        Ok(r) => Ok(ResolvedUpstream {
            base_url: r.base_url,
            api_key: r.api_key,
            is_byok: true,
            byok_profile_id: Some(r.profile_id),
        }),
        Err(ByokResolveError::NotFound) => Err(ApiError::NotFound("BYOK profile not found".into())),
        Err(ByokResolveError::Decrypt) => Err(ApiError::InternalError(
            "failed to decrypt BYOK credentials".into(),
        )),
        Err(ByokResolveError::Db(e)) => Err(ApiError::InternalError(format!("database error: {e}"))),
    }
}

fn chat_audit_metadata(
    stream: bool,
    is_byok: bool,
    byok_profile_id: Option<i64>,
    stream_extras: Option<serde_json::Map<String, serde_json::Value>>,
) -> serde_json::Value {
    let mut m = serde_json::Map::new();
    m.insert("stream".to_string(), stream.into());
    m.insert("is_byok".to_string(), is_byok.into());
    if let Some(id) = byok_profile_id {
        m.insert("byok_profile_id".to_string(), id.into());
    }
    if let Some(extra) = stream_extras {
        for (k, v) in extra {
            m.insert(k, v);
        }
    }
    serde_json::Value::Object(m)
}
