mod config;

use actix_cors::Cors;
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    dev::Service,
    body::{BoxBody, EitherBody},
    http::StatusCode as ActixStatusCode,
    web, App, HttpResponse, HttpServer,
};
use bytes::Bytes;
use futures_util::StreamExt;
use serde_json::Value;
use tracing::{error, info};

use crate::config::AppConfig;

#[derive(Clone)]
struct AppState {
    cfg: AppConfig,
    http: reqwest::Client,
}

async fn health() -> HttpResponse {
    HttpResponse::Ok().body("ok")
}

async fn not_found(req: actix_web::HttpRequest) -> HttpResponse {
    HttpResponse::NotFound().json(serde_json::json!({
        "error": {
            "message": "Not Found",
            "code": 404,
            "path": req.path(),
            "method": req.method().as_str()
        }
    }))
}

fn build_chat_completions_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');

    if base.ends_with("/chat/completions") {
        return base.to_string();
    }

    // Common patterns:
    // - OpenAI: https://api.openai.com/v1
    // - OpenRouter: https://openrouter.ai/api or https://openrouter.ai/api/v1
    if base.ends_with("/v1") {
        return format!("{base}/chat/completions");
    }
    if base.ends_with("/api") {
        return format!("{base}/v1/chat/completions");
    }

    format!("{base}/chat/completions")
}

async fn chat_completions(state: web::Data<AppState>, body: web::Bytes) -> HttpResponse {
    let upstream_url = build_chat_completions_url(&state.cfg.upstream.base_url);

    let mut req_builder = state
        .http
        .post(upstream_url.clone())
        .header("Authorization", format!("Bearer {}", state.cfg.upstream.api_key))
        .header("Content-Type", "application/json")
        .body(body.clone());

    // Detect stream=true to keep SSE content-type.
    let is_stream = serde_json::from_slice::<Value>(&body)
        .ok()
        .and_then(|v| v.get("stream").and_then(|s| s.as_bool()))
        .unwrap_or(false);

    // Propagate some optional OpenAI headers if present.
    // (We keep it minimal; compatibility can expand later.)
    if let Ok(org) = std::env::var("OPENAI_ORGANIZATION") {
        req_builder = req_builder.header("OpenAI-Organization", org);
    }
    if let Ok(project) = std::env::var("OPENAI_PROJECT") {
        req_builder = req_builder.header("OpenAI-Project", project);
    }

    let upstream_resp = match req_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            error!(error = %e, "upstream request failed");
            return HttpResponse::BadGateway().json(serde_json::json!({
                "error": {
                    "message": "Upstream request failed",
                    "type": "upstream_error"
                }
            }));
        }
    };

    let status = upstream_resp.status();
    let status = ActixStatusCode::from_u16(status.as_u16())
        .unwrap_or(ActixStatusCode::BAD_GATEWAY);

    if is_stream {
        let stream = upstream_resp
            .bytes_stream()
            .map(|chunk| chunk.map(Bytes::from).map_err(|e| {
                error!(error = %e, "upstream stream read failed");
                actix_web::error::ErrorBadGateway("upstream stream read failed")
            }));

        HttpResponse::build(status)
            .content_type("text/event-stream")
            .streaming(stream)
    } else {
        match upstream_resp.bytes().await {
            Ok(b) => HttpResponse::build(status)
                .content_type("application/json")
                .body(b),
            Err(e) => {
                error!(error = %e, "upstream response read failed");
                HttpResponse::BadGateway().json(serde_json::json!({
                    "error": {
                        "message": "Failed to read upstream response",
                        "type": "upstream_error"
                    }
                }))
            }
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,actix_web=info".into()),
        )
        .init();

    let cfg = config::load_config().unwrap_or_else(|e| {
        panic!("Failed to load config (config.toml or env): {e}");
    });

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .expect("failed to build http client");

    let host = cfg.server.host.clone();
    let port = cfg.server.port;
    info!(%host, %port, "starting server");

    let state = AppState { cfg, http };

    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .wrap_fn(|req: ServiceRequest, srv| {
                let method = req.method().clone();
                let path = req.path().to_string();
                let peer = req
                    .connection_info()
                    .realip_remote_addr()
                    .map(|s| s.to_string())
                    .or_else(|| req.peer_addr().map(|p| p.to_string()))
                    .unwrap_or_else(|| "-".to_string());
                let user_agent = req
                    .headers()
                    .get("user-agent")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("-")
                    .to_string();
                let start = std::time::Instant::now();

                let fut = srv.call(req);
                async move {
                    match fut.await {
                        Ok(res) => {
                            let status = res.status().as_u16();
                            let elapsed_ms = start.elapsed().as_millis();
                            info!(
                                method = %method,
                                path = %path,
                                status = status,
                                elapsed_ms = elapsed_ms,
                                peer = %peer,
                                user_agent = %user_agent,
                                "http access"
                            );
                            Ok::<ServiceResponse<EitherBody<BoxBody>>, actix_web::Error>(res)
                        }
                        Err(e) => {
                            let elapsed_ms = start.elapsed().as_millis();
                            error!(
                                method = %method,
                                path = %path,
                                elapsed_ms = elapsed_ms,
                                peer = %peer,
                                user_agent = %user_agent,
                                error = %e,
                                "http access error"
                            );
                            Err(e)
                        }
                    }
                }
            })
            .app_data(web::Data::new(state.clone()))
            .route("/healthz", web::get().to(health))
            .route("/v1/chat/completions", web::post().to(chat_completions))
            .default_service(web::route().to(not_found))
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}

