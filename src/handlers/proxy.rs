use actix_web::{
    http::StatusCode as ActixStatusCode, web, HttpRequest, HttpResponse, ResponseError,
};
use async_stream::stream;
use bytes::Bytes;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use reqwest::header as reqwest_header;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde_json::Value;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, info, warn};

use crate::{
    api_key_policy, auth, billing, byok, byok::ByokResolveError, db::ApiKeyAuthRow,
    errors::ApiError, key_concurrency, upstream, AppState,
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

static UPSTREAM_GET_HEADERS: Lazy<reqwest::header::HeaderMap> = Lazy::new(|| {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(reqwest_header::ACCEPT, "application/json".parse().unwrap());
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

/// `GET /v1/models` — same gateway API key and upstream resolution as chat; proxies to the upstream
/// OpenAI-compatible models list. Optional `model_allowlist` on the key filters the `data` array.
pub async fn list_models(
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, ApiError> {
    let api_key = auth::extract_bearer_token(&req)
        .ok_or_else(|| ApiError::Unauthorized("Invalid or missing API key".into()))?;
    if !api_key.starts_with("sk-or-v1-") {
        return Err(ApiError::Unauthorized(
            "Models list requires an sk-or-v1-* gateway API key".into(),
        ));
    }

    let auth_row = state.auth_service.get_api_key_auth(api_key)?;
    let token_id = auth_row.id;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

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

    let resolved = resolve_chat_upstream(&req, &state, &auth_row, None, now)?;

    let mut upstream_url = upstream::build_models_url(&resolved.base_url);
    if let Some(qs) = req.uri().query() {
        upstream_url.push('?');
        upstream_url.push_str(qs);
    }

    let req_builder = state
        .http
        .get(&upstream_url)
        .headers(UPSTREAM_GET_HEADERS.clone())
        .header(
            reqwest_header::AUTHORIZATION,
            format!("Bearer {}", resolved.api_key),
        );

    let upstream_resp = match req_builder.send().await {
        Ok(resp) => resp,
        Err(e) => {
            error!(error = %e, url = %upstream_url, "upstream models request failed");
            return Err(ApiError::InternalError("Upstream request failed".into()));
        }
    };

    let status = ActixStatusCode::from_u16(upstream_resp.status().as_u16())
        .unwrap_or(ActixStatusCode::BAD_GATEWAY);

    let body_bytes = upstream_resp.bytes().await.map_err(|e| {
        error!(error = %e, "reading upstream models body failed");
        ApiError::InternalError("Upstream request failed".into())
    })?;

    let code = status.as_u16();
    let out = if (200..300).contains(&code) {
        filter_models_list_json(&body_bytes, auth_row.model_allowlist.as_deref())?
    } else {
        body_bytes.to_vec()
    };

    Ok(HttpResponse::build(status)
        .content_type("application/json")
        .body(out))
}

fn filter_models_list_json(
    body: &[u8],
    model_allowlist_json: Option<&str>,
) -> Result<Vec<u8>, ApiError> {
    let raw = match model_allowlist_json.filter(|s| !s.is_empty()) {
        Some(r) => r,
        None => return Ok(body.to_vec()),
    };
    let allowed: Vec<String> = serde_json::from_str(raw)
        .map_err(|_| ApiError::InternalError("invalid model_allowlist on API key".into()))?;
    if allowed.is_empty() {
        return Ok(body.to_vec());
    }

    let mut v: Value = serde_json::from_slice(body).map_err(|_| {
        ApiError::InternalError("upstream models response is not valid JSON".into())
    })?;
    let Some(data) = v.get_mut("data").and_then(|d| d.as_array_mut()) else {
        return Ok(body.to_vec());
    };
    data.retain(|item| {
        item.get("id")
            .and_then(|x| x.as_str())
            .map(|id| allowed.iter().any(|a| a == id))
            .unwrap_or(false)
    });
    serde_json::to_vec(&v).map_err(|e| ApiError::InternalError(e.to_string()))
}

pub async fn chat_completions(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Bytes,
) -> Result<HttpResponse, ApiError> {
    let request_id = crate::audit::generate_request_id();
    let app_id = extract_app_id(&req);
    let thread_id = extract_thread_id(&req);
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
    let prompt_preview = crate::chat_prompt_preview::extract_chat_user_prompt_preview(&body);

    let auth_row = state.auth_service.get_api_key_auth(api_key)?;
    let token_id = auth_row.id;
    let user_id = auth_row.user_id;
    let key_team_id = auth_row.team_id;

    let audit_created_at = crate::audit::now_unix_secs();
    send_accept_phase_audit(
        &state,
        request_id.clone(),
        user_id,
        token_id,
        key_team_id,
        model.clone(),
        app_id.clone(),
        thread_id.clone(),
        is_stream,
        &req,
        audit_created_at,
        prompt_preview.clone(),
    )
    .await;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    if let Err(e) = state.user_service.ensure_monthly_quota(token_id, now) {
        let err = ApiError::from(e);
        audit_mark_rejected_api_error(&state, &request_id, &err, start).await;
        return Err(err);
    }

    if let Err(e) = state.user_service.ensure_monthly_spend_quota(token_id, now) {
        let err = ApiError::from(e);
        audit_mark_rejected_api_error(&state, &request_id, &err, start).await;
        return Err(err);
    }

    if let Err(m) =
        api_key_policy::check_model_allowlist(auth_row.model_allowlist.as_deref(), model.as_deref())
    {
        let err = ApiError::Forbidden(m.into());
        audit_mark_rejected_api_error(&state, &request_id, &err, start).await;
        return Err(err);
    }

    if let Some(ip) = api_key_policy::client_ip(&req) {
        if let Err(m) = api_key_policy::check_ip_allowlist(auth_row.ip_allowlist.as_deref(), ip) {
            let err = ApiError::Forbidden(m.into());
            audit_mark_rejected_api_error(&state, &request_id, &err, start).await;
            return Err(err);
        }
    } else if auth_row
        .ip_allowlist
        .as_ref()
        .map(|s| !s.is_empty())
        .unwrap_or(false)
    {
        let err = ApiError::Forbidden("cannot determine client IP for this API key policy".into());
        audit_mark_rejected_api_error(&state, &request_id, &err, start).await;
        return Err(err);
    }

    if let Err(e) = state.user_service.touch_api_key_last_used(token_id, now) {
        let err = ApiError::from(e);
        audit_mark_rejected_api_error(&state, &request_id, &err, start).await;
        return Err(err);
    }

    let session_key = crate::session_upstream::parse_session_key(thread_id.clone(), body.as_ref());
    let resolved = match resolve_chat_upstream(&req, &state, &auth_row, session_key.as_deref(), now)
    {
        Ok(r) => r,
        Err(e) => {
            audit_mark_rejected_api_error(&state, &request_id, &e, start).await;
            return Err(e);
        }
    };

    if let Err(e) = billing::check_can_start_chat(&state, user_id, resolved.is_byok) {
        audit_mark_rejected_api_error(&state, &request_id, &e, start).await;
        return Err(e);
    }
    let chat_slot = key_concurrency::acquire_chat_slot(
        &state.key_concurrency,
        token_id,
        auth_row.max_concurrent_requests,
    )
    .await;
    let client_request_headers_json = actix_request_headers_json(&req);
    debug!(
        %request_id,
        user_id,
        token_id,
        model = model.as_deref(),
        stream = is_stream,
        ?app_id,
        ?thread_id,
        is_byok = resolved.is_byok,
        "chat completions proxy request accepted"
    );
    let upstream_url = upstream::build_chat_completions_url(&resolved.base_url);
    let request_body_path =
        crate::audit::save_body_to_file(&state.audit_config, &request_id, "request", &body).ok();
    if let Some(ref p) = request_body_path {
        enqueue_audit_request_body_path(&state, request_id.clone(), p.clone()).await;
    }

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
                ?thread_id,
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
                    cached_prompt_tokens: None,
                    total_tokens: None,
                    cost: None,
                    latency_ms: Some(start.elapsed().as_millis() as i64),
                    reasoning_phase_ms: None,
                    app_id: app_id.clone(),
                    thread_id: thread_id.clone(),
                    finish_reason: None,
                    metadata: Some(chat_audit_metadata(
                        is_stream,
                        resolved.is_byok,
                        resolved.byok_profile_id,
                        None,
                        client_request_headers_json.clone(),
                        serde_json::json!({}),
                    )),
                    created_at: audit_created_at,
                    team_id: key_team_id,
                    prompt_preview: prompt_preview.clone(),
                },
            )
            .await;
            return Err(ApiError::InternalError("Upstream request failed".into()));
        }
    };

    let upstream_response_headers_json = reqwest_response_headers_json(&upstream_resp);

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
            ?thread_id,
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
                cached_prompt_tokens: None,
                total_tokens: None,
                cost: None,
                latency_ms: Some(start.elapsed().as_millis() as i64),
                reasoning_phase_ms: None,
                app_id: app_id.clone(),
                thread_id: thread_id.clone(),
                finish_reason: None,
                metadata: Some(chat_audit_metadata(
                    true,
                    resolved.is_byok,
                    resolved.byok_profile_id,
                    None,
                    client_request_headers_json.clone(),
                    upstream_response_headers_json.clone(),
                )),
                created_at: audit_created_at,
                team_id: key_team_id,
                prompt_preview: prompt_preview.clone(),
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
            ?thread_id,
            "chat completion proxied"
        );
        let st = state.clone();
        let status_ok = (200..300).contains(&status_i64);
        let stream_is_byok = resolved.is_byok;
        let stream_byok_id = resolved.byok_profile_id;
        let stream_hdr_req = client_request_headers_json.clone();
        let stream_hdr_resp = upstream_response_headers_json.clone();
        let stream = stream! {
            let _chat_slot = chat_slot;
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
                (None, None, None, None, None, None, None);
            let mut phase_timings = SseReasoningPhaseTimings::default();
            let mut upstream_stream = upstream_resp.bytes_stream();
            let mut buf: Vec<u8> = Vec::new();
            while let Some(item) = upstream_stream.next().await {
                match item {
                    Ok(chunk) => {
                        feed_sse_lines(
                            &mut buf,
                            chunk.as_ref(),
                            &mut usage_state,
                            &mut phase_timings,
                        );
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
                        flush_sse_lines(
                            &mut buf,
                            &mut usage_state,
                            &mut phase_timings,
                        );
                        if let Some((path, mut f)) = file_pair.take() {
                            let _ = f.flush().await;
                            let _ = f.shutdown().await;
                            let reasoning_phase_ms = phase_timings.finish_ms();
                            enqueue_stream_audit_completion(
                                &st,
                                StreamAuditCompletionJob {
                                    request_id: stream_request_id.clone(),
                                    response_body_path: path,
                                    usage: usage_state.clone(),
                                    latency_ms: start.elapsed().as_millis() as i64,
                                    reasoning_phase_ms,
                                    stream_completed: false,
                                    stream_aborted: true,
                                    error_message: Some("Upstream stream read failed".into()),
                                    is_byok: stream_is_byok,
                                    byok_profile_id: stream_byok_id,
                                    request_headers: stream_hdr_req.clone(),
                                    response_headers: stream_hdr_resp.clone(),
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
            flush_sse_lines(
                &mut buf,
                &mut usage_state,
                &mut phase_timings,
            );
            let reasoning_phase_ms = phase_timings.finish_ms();
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
                        reasoning_phase_ms,
                        stream_completed: true,
                        stream_aborted: false,
                        error_message: None,
                        is_byok: stream_is_byok,
                        byok_profile_id: stream_byok_id,
                        request_headers: stream_hdr_req.clone(),
                        response_headers: stream_hdr_resp.clone(),
                    },
                )
                .await;
            }
            if status_ok {
                if let Some(total) = usage_state.2 {
                    let _ = st.user_service.increment_quota_tokens(token_id, total);
                }
                billing::charge_chat_usage(
                    &st,
                    user_id,
                    token_id,
                    &stream_request_id,
                    model.as_deref(),
                    usage_state.0,
                    usage_state.1,
                    usage_state.5,
                    stream_is_byok,
                );
            }
        };

        Ok(HttpResponse::build(status)
            .content_type("text/event-stream")
            .streaming(stream))
    } else {
        let _chat_slot = chat_slot;
        let bytes = match upstream_resp.bytes().await {
            Ok(b) => b,
            Err(e) => {
                error!(
                    %request_id,
                    user_id,
                    token_id,
                    model = model.as_deref(),
                    upstream_status = status_i64,
                    ?app_id,
                    ?thread_id,
                    error = %e,
                    "upstream response read failed"
                );
                audit_mark_rejected_raw(
                    &state,
                    &request_id,
                    500,
                    format!("Failed to read upstream response: {e}"),
                    start,
                )
                .await;
                return Err(ApiError::InternalError(
                    "Failed to read upstream response".into(),
                ));
            }
        };
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
                cached_prompt_tokens: usage.6,
                total_tokens: usage.2,
                cost: usage.3,
                latency_ms: Some(start.elapsed().as_millis() as i64),
                reasoning_phase_ms: None,
                app_id: app_id.clone(),
                thread_id: thread_id.clone(),
                finish_reason: usage.4,
                metadata: Some(chat_audit_metadata(
                    false,
                    resolved.is_byok,
                    resolved.byok_profile_id,
                    None,
                    client_request_headers_json,
                    upstream_response_headers_json,
                )),
                created_at: audit_created_at,
                team_id: key_team_id,
                prompt_preview: prompt_preview.clone(),
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
            ?thread_id,
            prompt_tokens = usage.0,
            completion_tokens = usage.1,
            total_tokens = usage.2,
            "chat completion proxied"
        );

        if (200..300).contains(&status_i64) {
            if let Some(total) = usage.2 {
                let _ = state.user_service.increment_quota_tokens(token_id, total);
            }
            billing::charge_chat_usage(
                &state,
                user_id,
                token_id,
                &log_request_id,
                model.as_deref(),
                usage.0,
                usage.1,
                usage.5,
                resolved.is_byok,
            );
        }

        Ok(HttpResponse::build(status)
            .content_type("application/json")
            .body(bytes))
    }
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
    let (p, c, t, co, fr, up, cp) = parse_usage_cost_and_finish(chunk);
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
    if up.is_some() {
        usage.5 = up;
    }
    if cp.is_some() {
        usage.6 = cp;
    }
}

#[derive(Default)]
struct SseReasoningPhaseTimings {
    first_reasoning: Option<std::time::Instant>,
    first_content: Option<std::time::Instant>,
}

impl SseReasoningPhaseTimings {
    fn finish_ms(&self) -> Option<i64> {
        match (self.first_reasoning, self.first_content) {
            (Some(a), Some(b)) if b >= a => Some(b.duration_since(a).as_millis() as i64),
            _ => None,
        }
    }
}

fn parse_sse_data_line_reasoning_phase(line: &[u8], timings: &mut SseReasoningPhaseTimings) {
    let s = String::from_utf8_lossy(line);
    let t = s.trim_end();
    let Some(rest) = t.strip_prefix("data: ") else {
        return;
    };
    let rest = rest.trim();
    if rest.is_empty() || rest == "[DONE]" {
        return;
    }
    let Ok(v) = serde_json::from_str::<Value>(rest) else {
        return;
    };
    let Some(choices) = v.get("choices").and_then(|c| c.as_array()) else {
        return;
    };
    let Some(ch0) = choices.first() else {
        return;
    };
    let Some(delta) = ch0.get("delta") else {
        return;
    };
    let now = std::time::Instant::now();
    if timings.first_reasoning.is_none() {
        let r = delta
            .get("reasoning_content")
            .and_then(|v| v.as_str())
            .or_else(|| delta.get("reasoning").and_then(|v| v.as_str()))
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if r.is_some() {
            timings.first_reasoning = Some(now);
        }
    }
    if timings.first_content.is_none() {
        let has_content = match delta.get("content") {
            Some(Value::String(s)) => !s.trim().is_empty(),
            Some(Value::Array(arr)) => !arr.is_empty(),
            Some(Value::Null) | None => false,
            Some(_) => true,
        };
        if has_content {
            timings.first_content = Some(now);
        }
    }
}

fn feed_sse_lines(
    buf: &mut Vec<u8>,
    chunk: &[u8],
    usage: &mut UsageTokensCostFinish,
    phase: &mut SseReasoningPhaseTimings,
) {
    buf.extend_from_slice(chunk);
    while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
        let line: Vec<u8> = buf.drain(..=pos).collect();
        parse_sse_data_line_merge_usage(&line, usage);
        parse_sse_data_line_reasoning_phase(&line, phase);
    }
}

fn flush_sse_lines(
    buf: &mut Vec<u8>,
    usage: &mut UsageTokensCostFinish,
    phase: &mut SseReasoningPhaseTimings,
) {
    if buf.is_empty() {
        return;
    }
    let line = std::mem::take(buf);
    parse_sse_data_line_merge_usage(&line, usage);
    parse_sse_data_line_reasoning_phase(&line, phase);
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

fn extract_thread_id(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("x-thread-id")
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
    Option<Decimal>,
    Option<i64>,
);

struct StreamAuditCompletionJob {
    request_id: String,
    response_body_path: String,
    usage: UsageTokensCostFinish,
    latency_ms: i64,
    reasoning_phase_ms: Option<i64>,
    stream_completed: bool,
    stream_aborted: bool,
    error_message: Option<String>,
    is_byok: bool,
    byok_profile_id: Option<i64>,
    request_headers: serde_json::Value,
    response_headers: serde_json::Value,
}

/// OpenAI-style chat completion JSON: `usage` + `choices[0].finish_reason` + upstream USD for billing.
fn parse_usage_cost_and_finish(body: &[u8]) -> UsageTokensCostFinish {
    let value = match serde_json::from_slice::<Value>(body) {
        Ok(v) => v,
        Err(_) => return (None, None, None, None, None, None, None),
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
    let cached_prompt_tokens = usage
        .and_then(|u| u.get("prompt_tokens_details"))
        .and_then(|d| d.get("cached_tokens"))
        .and_then(|v| v.as_i64());
    let upstream_usd = crate::billing::upstream_cost_usd_from_response(&value);
    // Audit `cost`: top-level `cost` OR same USD as billing (`cost_details` / nested `usage`).
    let cost = value
        .get("cost")
        .and_then(|v| v.as_f64())
        .or_else(|| upstream_usd.as_ref().and_then(ToPrimitive::to_f64));
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
        upstream_usd,
        cached_prompt_tokens,
    )
}

fn accept_phase_audit_metadata(req: &HttpRequest, is_stream: bool) -> serde_json::Value {
    let mut m = serde_json::Map::new();
    m.insert("accept_phase".to_string(), true.into());
    m.insert("stream".to_string(), is_stream.into());
    m.insert("is_byok".to_string(), false.into());
    m.insert(
        "request_headers".to_string(),
        actix_request_headers_json(req),
    );
    m.insert("response_headers".to_string(), serde_json::json!({}));
    serde_json::Value::Object(m)
}

#[allow(clippy::too_many_arguments)]
async fn send_accept_phase_audit(
    state: &web::Data<AppState>,
    request_id: String,
    user_id: i64,
    token_id: i64,
    key_team_id: Option<i64>,
    model: Option<String>,
    app_id: Option<String>,
    thread_id: Option<String>,
    is_stream: bool,
    req: &HttpRequest,
    created_at: i64,
    prompt_preview: Option<String>,
) {
    send_audit_record(
        state,
        crate::audit::AuditRecord {
            request_id,
            user_id: Some(user_id),
            token_id: Some(token_id),
            channel_id: None,
            model,
            request_type: Some("chat".to_string()),
            request_body_path: None,
            response_body_path: None,
            status_code: None,
            error_message: None,
            prompt_tokens: None,
            completion_tokens: None,
            cached_prompt_tokens: None,
            total_tokens: None,
            cost: None,
            latency_ms: None,
            reasoning_phase_ms: None,
            app_id,
            thread_id,
            finish_reason: None,
            metadata: Some(accept_phase_audit_metadata(req, is_stream)),
            created_at,
            team_id: key_team_id,
            prompt_preview,
        },
    )
    .await;
}

async fn enqueue_audit_request_body_path(
    state: &web::Data<AppState>,
    request_id: String,
    path: String,
) {
    if let Err(err) = state
        .audit_sender
        .send(crate::audit::AuditMessage::RequestBodyPath { request_id, path })
        .await
    {
        error!(error = %err, "failed to enqueue audit request body path");
    }
}

async fn audit_enqueue_rejected(
    state: &web::Data<AppState>,
    request_id: &str,
    status_code: i64,
    error_message: String,
    latency_ms: i64,
) {
    if let Err(err) = state
        .audit_sender
        .send(crate::audit::AuditMessage::Rejected {
            request_id: request_id.to_string(),
            status_code,
            error_message,
            latency_ms,
        })
        .await
    {
        error!(error = %err, "failed to enqueue audit rejected update");
    }
}

async fn audit_mark_rejected_api_error(
    state: &web::Data<AppState>,
    request_id: &str,
    err: &ApiError,
    start: std::time::Instant,
) {
    let status_code = ResponseError::status_code(err).as_u16() as i64;
    let error_message = err.to_string();
    let latency_ms = start.elapsed().as_millis() as i64;
    audit_enqueue_rejected(state, request_id, status_code, error_message, latency_ms).await;
}

async fn audit_mark_rejected_raw(
    state: &web::Data<AppState>,
    request_id: &str,
    status_code: i64,
    error_message: String,
    start: std::time::Instant,
) {
    let latency_ms = start.elapsed().as_millis() as i64;
    audit_enqueue_rejected(state, request_id, status_code, error_message, latency_ms).await;
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
        reasoning_phase_ms,
        stream_completed,
        stream_aborted,
        error_message,
        is_byok,
        byok_profile_id,
        request_headers,
        response_headers,
    } = job;
    let mut stream_extra = serde_json::Map::new();
    stream_extra.insert("stream_completed".into(), stream_completed.into());
    stream_extra.insert("stream_aborted".into(), stream_aborted.into());
    stream_extra.insert("response_body_format".into(), "text/event-stream".into());
    let metadata = chat_audit_metadata(
        true,
        is_byok,
        byok_profile_id,
        Some(stream_extra),
        request_headers,
        response_headers,
    );
    let update = crate::audit::AuditStreamCompletionUpdate {
        request_id,
        response_body_path,
        prompt_tokens: usage.0,
        completion_tokens: usage.1,
        cached_prompt_tokens: usage.6,
        total_tokens: usage.2,
        cost: usage.3,
        finish_reason: usage.4,
        latency_ms,
        reasoning_phase_ms,
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

fn resolve_picked_upstream(
    state: &web::Data<AppState>,
    auth_row: &ApiKeyAuthRow,
    pick: crate::session_upstream::PickedUpstream,
) -> Result<ResolvedUpstream, ApiError> {
    let user_id = auth_row.user_id;
    let api_key_team_id = auth_row.team_id;
    match pick {
        crate::session_upstream::PickedUpstream::Platform => Ok(ResolvedUpstream {
            base_url: state.cfg.upstream.base_url.clone(),
            api_key: state.cfg.upstream.api_key.clone(),
            is_byok: false,
            byok_profile_id: None,
        }),
        crate::session_upstream::PickedUpstream::Byok(pid) => {
            if pid <= 0 {
                return Err(ApiError::BadRequest(
                    "invalid BYOK profile id in session pool".into(),
                ));
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
                Err(ByokResolveError::NotFound) => {
                    Err(ApiError::NotFound("BYOK profile not found".into()))
                }
                Err(ByokResolveError::Decrypt) => Err(ApiError::InternalError(
                    "failed to decrypt BYOK credentials".into(),
                )),
                Err(ByokResolveError::Db(e)) => {
                    Err(ApiError::InternalError(format!("database error: {e}")))
                }
            }
        }
    }
}

fn resolve_chat_upstream(
    req: &HttpRequest,
    state: &web::Data<AppState>,
    auth_row: &ApiKeyAuthRow,
    session_key: Option<&str>,
    now: i64,
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

    if auth_row.session_affinity_enabled {
        if let Ok(pool) = crate::session_upstream::parse_upstream_pool_json(
            auth_row.upstream_pool_json.as_deref(),
        ) {
            if !pool.is_empty() {
                if let Some(sk) = session_key.filter(|s| !s.is_empty()) {
                    let mut conn = state
                        .db
                        .get()
                        .map_err(|_| ApiError::InternalError("database pool unavailable".into()))?;
                    let pick = crate::session_upstream::pick_session_upstream(
                        &mut conn,
                        auth_row.id,
                        &pool,
                        sk,
                        now,
                    )
                    .map_err(|e| {
                        ApiError::InternalError(format!("session upstream binding: {e}"))
                    })?;
                    return resolve_picked_upstream(state, auth_row, pick);
                }
            }
        }
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
        return Err(ApiError::BadRequest(
            "invalid default BYOK profile id".into(),
        ));
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
        Err(ByokResolveError::Db(e)) => {
            Err(ApiError::InternalError(format!("database error: {e}")))
        }
    }
}

fn chat_audit_metadata(
    stream: bool,
    is_byok: bool,
    byok_profile_id: Option<i64>,
    stream_extras: Option<serde_json::Map<String, serde_json::Value>>,
    request_headers: serde_json::Value,
    response_headers: serde_json::Value,
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
    m.insert("request_headers".to_string(), request_headers);
    m.insert("response_headers".to_string(), response_headers);
    serde_json::Value::Object(m)
}

/// HTTP header names whose values must not be stored in audit metadata.
const SENSITIVE_AUDIT_HEADER_NAMES: &[&str] = &[
    "authorization",
    "cookie",
    "set-cookie",
    "proxy-authorization",
    "x-api-key",
];

fn redact_audit_header_value(header_name_lower: &str, value: &str) -> String {
    if SENSITIVE_AUDIT_HEADER_NAMES.contains(&header_name_lower) {
        return "[REDACTED]".to_string();
    }
    value.to_string()
}

fn merge_audit_header_line(
    map: &mut serde_json::Map<String, serde_json::Value>,
    key: String,
    value: String,
) {
    use serde_json::Value;
    match map.get_mut(&key) {
        Some(Value::String(prev)) => {
            let joined = format!("{prev}, {value}");
            *prev = joined;
        }
        Some(_) => {
            map.insert(key, Value::String(value));
        }
        None => {
            map.insert(key, Value::String(value));
        }
    }
}

fn actix_request_headers_json(req: &HttpRequest) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (name, value) in req.headers().iter() {
        let key = name.as_str().to_ascii_lowercase();
        let raw = value.to_str().unwrap_or("");
        let v = redact_audit_header_value(&key, raw);
        merge_audit_header_line(&mut map, key, v);
    }
    serde_json::Value::Object(map)
}

fn reqwest_response_headers_json(resp: &reqwest::Response) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (name, value) in resp.headers().iter() {
        let key = name.as_str().to_ascii_lowercase();
        let raw = value.to_str().unwrap_or("");
        let v = redact_audit_header_value(&key, raw);
        merge_audit_header_line(&mut map, key, v);
    }
    serde_json::Value::Object(map)
}

#[cfg(test)]
mod filter_models_list_tests {
    use super::filter_models_list_json;

    #[test]
    fn allowlist_none_passthrough() {
        let body = br#"{"object":"list","data":[{"id":"a"},{"id":"b"}]}"#;
        let out = filter_models_list_json(body, None).unwrap();
        assert_eq!(out, body);
    }

    #[test]
    fn allowlist_empty_array_passthrough() {
        let body = br#"{"object":"list","data":[{"id":"a"}]}"#;
        let out = filter_models_list_json(body, Some("[]")).unwrap();
        assert_eq!(out, body);
    }

    #[test]
    fn allowlist_filters_data() {
        let body = br#"{"object":"list","data":[{"id":"keep"},{"id":"drop"}]}"#;
        let out = filter_models_list_json(body, Some(r#"["keep"]"#)).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["data"].as_array().unwrap().len(), 1);
        assert_eq!(v["data"][0]["id"], "keep");
    }

    #[test]
    fn missing_data_array_passthrough() {
        let body = br#"{"error":{"message":"x"}}"#;
        let out = filter_models_list_json(body, Some(r#"["a"]"#)).unwrap();
        assert_eq!(out, body);
    }
}

#[cfg(test)]
mod parse_usage_cost_tests {
    use super::parse_usage_cost_and_finish;
    use serde_json::json;

    fn parse(s: &serde_json::Value) -> super::UsageTokensCostFinish {
        parse_usage_cost_and_finish(s.to_string().as_bytes())
    }

    #[test]
    fn audit_cost_uses_top_level_cost_when_present() {
        let v = json!({
            "usage": { "prompt_tokens": 1, "completion_tokens": 2, "total_tokens": 3 },
            "cost": 0.05,
            "cost_details": { "upstream_inference_cost": "0.0000189" }
        });
        let (_, _, _, cost, _, upstream, _) = parse(&v);
        assert!((cost.unwrap() - 0.05).abs() < 1e-12);
        assert_eq!(upstream.unwrap().to_string(), "0.0000189");
    }

    #[test]
    fn audit_cost_falls_back_to_billing_usd_when_no_root_cost() {
        let v = json!({
            "usage": { "prompt_tokens": 1, "completion_tokens": 2, "total_tokens": 3 },
            "cost_details": { "upstream_inference_cost": "0.0000189" }
        });
        let (_, _, _, cost, _, upstream, _) = parse(&v);
        assert!(cost.is_some());
        assert!((cost.unwrap() - 0.0000189).abs() < 1e-15);
        assert_eq!(upstream.unwrap().to_string(), "0.0000189");
    }

    #[test]
    fn audit_cost_from_openrouter_style_usage_block() {
        let v = json!({
            "usage": {
                "prompt_tokens": 42,
                "completion_tokens": 79,
                "total_tokens": 121,
                "cost": 0.0000252,
                "cost_details": {
                    "upstream_inference_cost": 0.0000252
                }
            }
        });
        let (_, _, _, cost, _, upstream, _) = parse(&v);
        assert!((cost.unwrap() - 0.0000252).abs() < 1e-15);
        assert_eq!(upstream.unwrap().to_string(), "0.0000252");
    }

    #[test]
    fn cached_prompt_tokens_from_usage_details() {
        let v = json!({
            "usage": {
                "prompt_tokens": 100,
                "prompt_tokens_details": { "cached_tokens": 80 },
                "completion_tokens": 5,
                "total_tokens": 105
            }
        });
        let (_, _, _, _, _, _, cached) = parse(&v);
        assert_eq!(cached, Some(80));
    }

    #[test]
    fn invalid_json_returns_empty_tuple() {
        let t = parse_usage_cost_and_finish(b"not json {{{");
        assert!(t.0.is_none() && t.1.is_none() && t.2.is_none() && t.3.is_none());
        assert!(t.4.is_none() && t.5.is_none() && t.6.is_none());
    }
}

#[cfg(test)]
mod sse_reasoning_phase_tests {
    use super::{parse_sse_data_line_reasoning_phase, SseReasoningPhaseTimings};

    #[test]
    fn records_duration_between_reasoning_and_content_deltas() {
        let mut t = SseReasoningPhaseTimings::default();
        parse_sse_data_line_reasoning_phase(
            br#"data: {"choices":[{"delta":{"reasoning_content":"x"}}]}"#,
            &mut t,
        );
        assert!(t.first_reasoning.is_some());
        assert!(t.first_content.is_none());
        parse_sse_data_line_reasoning_phase(
            br#"data: {"choices":[{"delta":{"content":"hi"}}]}"#,
            &mut t,
        );
        assert!(t.first_content.is_some());
        let ms = t.finish_ms().expect("reasoning span");
        assert!(ms >= 0);
    }
}
