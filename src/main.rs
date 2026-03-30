pub mod config;

use actix_cors::Cors;
use actix_web::{
    body::{BoxBody, EitherBody},
    dev::Service,
    dev::{ServiceRequest, ServiceResponse},
    http::StatusCode as ActixStatusCode,
    web, App, HttpRequest, HttpResponse, HttpServer,
};
use futures_util::StreamExt;
use rusqlite::{params, Connection, ErrorCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info};
use uuid::Uuid;

use crate::config::AppConfig;

const MIGRATIONS: [(&str, &str); 1] = [
    (
        "0001_create_users.sql",
        include_str!("../migrations/0001_create_users.sql"),
    ),
];

fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS migration_versions (
            version TEXT PRIMARY KEY,
            applied_at INTEGER NOT NULL
        )",
        [],
    )?;

    for (version, sql) in MIGRATIONS.iter() {
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM migration_versions WHERE version = ?1)",
            params![*version],
            |row| row.get(0),
        )?;

        if exists {
            continue;
        }

        conn.execute_batch(sql)?;

        let applied_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO migration_versions (version, applied_at) VALUES (?1, ?2)",
            params![*version, applied_at],
        )?;
    }

    Ok(())
}

pub type DbConn = Arc<Mutex<Connection>>;

#[derive(Debug, Clone)]
pub struct AppState {
    pub cfg: AppConfig,
    pub http: reqwest::Client,
    pub db: DbConn,
}

#[derive(Deserialize)]
struct CreateUserRequest {
    username: String,
}

#[derive(Serialize)]
struct CreateUserResponse {
    username: String,
    api_key: String,
    created_at: u64,
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

async fn chat_completions(req: HttpRequest, state: web::Data<AppState>, body: web::Bytes) -> HttpResponse {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let valid_api_key = auth_header
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|key| {
            let db = state.db.lock().expect("db lock");
            db.query_row(
                "SELECT 1 FROM users WHERE api_key = ?1",
                params![*key],
                |_| Ok(()),
            )
            .is_ok()
        });

    if valid_api_key.is_none() {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "error": {
                "message": "Invalid or missing API key",
                "type": "authentication_error"
            }
        }));
    }

    let upstream_url = build_chat_completions_url(&state.cfg.upstream.base_url);

    let mut req_builder = state
        .http
        .post(upstream_url.clone())
        .header(
            "Authorization",
            format!("Bearer {}", state.cfg.upstream.api_key),
        )
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
    let status = ActixStatusCode::from_u16(status.as_u16()).unwrap_or(ActixStatusCode::BAD_GATEWAY);

    if is_stream {
        let stream = upstream_resp.bytes_stream().map(|chunk| {
            chunk.map_err(|e| {
                error!(error = %e, "upstream stream read failed");
                actix_web::error::ErrorBadGateway("upstream stream read failed")
            })
        });

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

    let db = Connection::open(&cfg.sqlite.path)
        .unwrap_or_else(|e| panic!("failed to open sqlite database: {e}"));
    run_migrations(&db).expect("failed to run database migrations");

    let host = cfg.server.host.clone();
    let port = cfg.server.port;
    info!(%host, %port, "starting server");

    let state = AppState {
        cfg,
        http,
        db: Arc::new(Mutex::new(db)),
    };

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
            .route("/users", web::post().to(create_user))
            .route("/v1/chat/completions", web::post().to(chat_completions))
            .default_service(web::route().to(not_found))
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}

async fn create_user(state: web::Data<AppState>, req: web::Json<CreateUserRequest>) -> HttpResponse {
    let username = req.username.trim();
    if username.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": {
                "message": "username is required",
                "type": "validation_error"
            }
        }));
    }

    let api_key = Uuid::new_v4().to_string();
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let conn = state.db.lock().expect("db lock");
    let result = conn.execute(
        "INSERT INTO users (username, api_key, created_at) VALUES (?1, ?2, ?3)",
        params![username, api_key, created_at],
    );

    if let Err(err) = result {
        if let rusqlite::Error::SqliteFailure(err_code, _) = &err {
            if err_code.code == ErrorCode::ConstraintViolation {
                return HttpResponse::Conflict().json(serde_json::json!({
                    "error": {
                        "message": "username already exists",
                        "type": "conflict_error"
                    }
                }));
            }
        }

        error!(error = %err, "failed to create user");
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": {
                "message": "Failed to create user",
                "type": "internal_error"
            }
        }));
    }

    HttpResponse::Created().json(CreateUserResponse {
        username: username.to_string(),
        api_key,
        created_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, HttpResponse, HttpServer, web};
    use actix_web::http::StatusCode as ActixStatusCode;
    use reqwest::StatusCode as ReqwestStatusCode;
    use rusqlite::Connection;
    use serde_json::json;
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    fn test_state() -> AppState {
        let cfg = AppConfig {
            server: config::ServerConfig {
                host: "127.0.0.1".into(),
                port: 0,
            },
            upstream: config::UpstreamConfig {
                base_url: "https://api.openai.com/v1".into(),
                api_key: "test".into(),
            },
            sqlite: config::SqliteConfig {
                path: ":memory:".into(),
            },
        };

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("failed to build http client");

        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        run_migrations(&conn).expect("run migrations");

        AppState {
            cfg,
            http,
            db: Arc::new(Mutex::new(conn)),
        }
    }

    #[tokio::test]
    async fn create_user_persists_to_sqlite() {
        let state = web::Data::new(test_state());
        let req = web::Json(CreateUserRequest {
            username: "alice".into(),
        });

        let resp = create_user(state.clone(), req).await;
        assert_eq!(resp.status(), ActixStatusCode::CREATED);

        let conn = state.db.lock().expect("db lock");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM users WHERE username = ?1", params!["alice"], |row| row.get(0))
            .expect("query count");
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn create_user_duplicate_username_returns_conflict() {
        let state = web::Data::new(test_state());
        let req1 = web::Json(CreateUserRequest {
            username: "bob".into(),
        });
        let req2 = web::Json(CreateUserRequest {
            username: "bob".into(),
        });

        let resp1 = create_user(state.clone(), req1).await;
        assert_eq!(resp1.status(), ActixStatusCode::CREATED);

        let resp2 = create_user(state, req2).await;
        assert_eq!(resp2.status(), ActixStatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn e2e_create_user_and_chat_completions() {
        let upstream_listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind upstream");
        let upstream_addr = upstream_listener.local_addr().expect("local addr");
        let upstream_server = HttpServer::new(|| {
            App::new().route("/v1/chat/completions", web::post().to(|| async {
                HttpResponse::Ok().json(json!({
                    "id": "chatcmpl-123",
                    "object": "chat.completion",
                    "choices": [{"message": {"role": "assistant", "content": "Hello from mock"}}]
                }))
            }))
        })
        .listen(upstream_listener)
        .expect("listen upstream")
        .run();
        let _upstream_handle = tokio::spawn(upstream_server);

        let cfg = AppConfig {
            server: config::ServerConfig {
                host: "127.0.0.1".into(),
                port: 0,
            },
            upstream: config::UpstreamConfig {
                base_url: format!("http://{}", upstream_addr),
                api_key: "dummy".into(),
            },
            sqlite: config::SqliteConfig {
                path: ":memory:".into(),
            },
        };

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("failed to build http client");

        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        run_migrations(&conn).expect("run migrations");

        let state = AppState {
            cfg,
            http,
            db: Arc::new(Mutex::new(conn)),
        };
        let app_state = state.clone();

        let app_listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind app");
        let app_addr = app_listener.local_addr().expect("local addr");
        let app_server = HttpServer::new(move || {
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
                .app_data(web::Data::new(app_state.clone()))
                .route("/healthz", web::get().to(health))
                .route("/users", web::post().to(create_user))
                .route("/v1/chat/completions", web::post().to(chat_completions))
                .default_service(web::route().to(not_found))
        })
        .listen(app_listener)
        .expect("listen app")
        .run();
        let _app_handle = tokio::spawn(app_server);
        let app_url = format!("http://{}", app_addr);
        let client = reqwest::Client::new();

        let create_resp = client
            .post(format!("{}/users", app_url))
            .json(&json!({"username": "testuser"}))
            .send()
            .await
            .expect("create user request");

        assert_eq!(create_resp.status(), ReqwestStatusCode::CREATED);
        let create_body: serde_json::Value = create_resp.json().await.expect("parse create response");
        let api_key = create_body["api_key"].as_str().expect("api_key exists");
        assert!(!api_key.is_empty());

        let conn = state.db.lock().expect("db lock");
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE username = ?1",
                params!["testuser"],
                |row| row.get(0),
            )
            .expect("query count");
        assert_eq!(count, 1);

        let chat_resp = client
            .post(format!("{}/v1/chat/completions", app_url))
            .bearer_auth(api_key)
            .json(&json!({
                "model": "gpt-4",
                "messages": [{"role": "user", "content": "hi"}],
            }))
            .send()
            .await
            .expect("chat request");

        assert_eq!(chat_resp.status(), ReqwestStatusCode::OK);
        let chat_body: serde_json::Value = chat_resp.json().await.expect("parse chat response");
        assert_eq!(chat_body["choices"][0]["message"]["content"], "Hello from mock");
    }
}
