pub mod config;

use actix_cors::Cors;
use actix_web::{
    body::{BoxBody, EitherBody},
    dev::Service,
    dev::{ServiceRequest, ServiceResponse},
    http::{header, StatusCode as ActixStatusCode},
    web, App, HttpRequest, HttpResponse, HttpServer,
};
use futures_util::StreamExt;
use reqwest::header as reqwest_header;
use rusqlite::{params, Connection, ErrorCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{error, info};
use uuid::Uuid;

use crate::config::AppConfig;

const MIGRATIONS: [(&str, &str); 1] = [(
    "0001_create_users.sql",
    include_str!("../migrations/0001_create_users.sql"),
)];

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

fn api_error(status: ActixStatusCode, error_type: &str, message: &str) -> HttpResponse {
    HttpResponse::build(status).json(json!({
        "error": {
            "message": message,
            "type": error_type,
        }
    }))
}

fn bad_request(message: &str) -> HttpResponse {
    api_error(ActixStatusCode::BAD_REQUEST, "validation_error", message)
}

fn unauthorized(message: &str) -> HttpResponse {
    api_error(
        ActixStatusCode::UNAUTHORIZED,
        "authentication_error",
        message,
    )
}

fn conflict(message: &str) -> HttpResponse {
    api_error(ActixStatusCode::CONFLICT, "conflict_error", message)
}

fn internal_error(message: &str) -> HttpResponse {
    api_error(
        ActixStatusCode::INTERNAL_SERVER_ERROR,
        "internal_error",
        message,
    )
}

fn not_found_error(message: &str) -> HttpResponse {
    api_error(ActixStatusCode::NOT_FOUND, "not_found_error", message)
}

fn extract_bearer_token(req: &HttpRequest) -> Option<&str> {
    req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

fn lock_db(db: &DbConn) -> MutexGuard<'_, Connection> {
    db.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
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

fn create_api_key() -> String {
    Uuid::new_v4().to_string()
}

fn validate_api_key(conn: &Connection, api_key: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM api_keys WHERE api_key = ?1 AND revoked = 0",
        params![api_key],
        |_| Ok(()),
    )
    .is_ok()
}

fn find_user_id(conn: &Connection, username: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT id FROM users WHERE username = ?1",
        params![username],
        |row| row.get(0),
    )
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

    format!("{base}/v1/chat/completions")
}

async fn chat_completions(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Bytes,
) -> HttpResponse {
    let api_key = match extract_bearer_token(&req) {
        Some(key) => key,
        None => return unauthorized("Invalid or missing API key"),
    };

    let db = lock_db(&state.db);
    if !validate_api_key(&db, api_key) {
        return unauthorized("Invalid or missing API key");
    }

    let upstream_url = build_chat_completions_url(&state.cfg.upstream.base_url);

    let mut req_builder = state
        .http
        .post(upstream_url.clone())
        .header(
            reqwest_header::AUTHORIZATION,
            format!("Bearer {}", state.cfg.upstream.api_key),
        )
        .header(reqwest_header::CONTENT_TYPE, "application/json")
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
            return internal_error("Upstream request failed");
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
                internal_error("Failed to read upstream response")
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
            .route(
                "/users/{username}/keys",
                web::post().to(create_user_api_key),
            )
            .route("/v1/chat/completions", web::post().to(chat_completions))
            .default_service(web::route().to(not_found))
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}

async fn create_user(
    state: web::Data<AppState>,
    req: web::Json<CreateUserRequest>,
) -> HttpResponse {
    let username = req.username.trim();
    if username.is_empty() {
        return bad_request("username is required");
    }

    let api_key = Uuid::new_v4().to_string();
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut conn = lock_db(&state.db);
    let tx = match conn.transaction() {
        Ok(tx) => tx,
        Err(err) => {
            error!(error = %err, "failed to begin transaction");
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": {
                    "message": "Failed to create user",
                    "type": "internal_error"
                }
            }));
        }
    };

    let result = tx.execute(
        "INSERT INTO users (username, created_at) VALUES (?1, ?2)",
        params![username, created_at],
    );

    if let Err(err) = result {
        if let rusqlite::Error::SqliteFailure(err_code, _) = &err {
            if err_code.code == ErrorCode::ConstraintViolation {
                return conflict("username already exists");
            }
        }

        error!(error = %err, "failed to create user");
        return internal_error("Failed to create user");
    }

    let user_id = tx.last_insert_rowid();
    let key_result = tx.execute(
        "INSERT INTO api_keys (user_id, api_key, created_at) VALUES (?1, ?2, ?3)",
        params![user_id, api_key, created_at],
    );

    if let Err(err) = key_result {
        error!(error = %err, "failed to create user api key");
        return internal_error("Failed to create user");
    }

    if let Err(err) = tx.commit() {
        error!(error = %err, "failed to commit transaction");
        return internal_error("Failed to create user");
    }

    HttpResponse::Created().json(CreateUserResponse {
        username: username.to_string(),
        api_key,
        created_at,
    })
}

async fn create_user_api_key(state: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let username = path.into_inner();
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let api_key = create_api_key();

    let conn = lock_db(&state.db);
    let user_id = match find_user_id(&conn, &username) {
        Ok(id) => id,
        Err(_) => return not_found_error("user not found"),
    };

    if let Err(err) = conn.execute(
        "INSERT INTO api_keys (user_id, api_key, created_at) VALUES (?1, ?2, ?3)",
        params![user_id, api_key, created_at],
    ) {
        error!(error = %err, "failed to create api key for user");
        return internal_error("Failed to create api key");
    }

    HttpResponse::Created().json(CreateUserResponse {
        username,
        api_key,
        created_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode as ActixStatusCode;
    use actix_web::{web, App, HttpResponse, HttpServer};
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

        let conn = lock_db(&state.db);
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM users WHERE username = ?1",
                params!["alice"],
                |row| row.get(0),
            )
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
    async fn create_additional_api_key_for_existing_user() {
        let state = web::Data::new(test_state());
        let req = web::Json(CreateUserRequest {
            username: "charlie".into(),
        });
        let resp = create_user(state.clone(), req).await;
        assert_eq!(resp.status(), ActixStatusCode::CREATED);

        let path = web::Path::from("charlie".to_string());
        let create_key_resp = create_user_api_key(state.clone(), path).await;
        assert_eq!(create_key_resp.status(), ActixStatusCode::CREATED);

        let conn = lock_db(&state.db);
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM api_keys WHERE user_id = (SELECT id FROM users WHERE username = ?1)",
                params!["charlie"],
                |row| row.get(0),
            )
            .expect("query key count");
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn e2e_create_user_and_chat_completions() {
        let upstream_listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind upstream");
        let upstream_addr = upstream_listener.local_addr().expect("local addr");
        let upstream_server = HttpServer::new(|| {
            App::new().route(
                "/v1/chat/completions",
                web::post().to(|| async {
                    HttpResponse::Ok().json(json!({
                    "id": "chatcmpl-123",
                    "object": "chat.completion",
                    "choices": [{"message": {"role": "assistant", "content": "Hello from mock"}}]
                }))
                }),
            )
        })
        .listen(upstream_listener)
        .expect("listen upstream")
        .run();
        let upstream_handle = tokio::spawn(upstream_server);

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
                .route(
                    "/users/{username}/keys",
                    web::post().to(create_user_api_key),
                )
                .route("/v1/chat/completions", web::post().to(chat_completions))
                .default_service(web::route().to(not_found))
        })
        .listen(app_listener)
        .expect("listen app")
        .run();
        let app_handle = tokio::spawn(app_server);
        let app_url = format!("http://{}", app_addr);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("failed to build test client");

        let create_resp = client
            .post(format!("{}/users", app_url))
            .json(&json!({"username": "testuser"}))
            .send()
            .await
            .expect("create user request");

        assert_eq!(create_resp.status(), ReqwestStatusCode::CREATED);
        let create_body: serde_json::Value =
            create_resp.json().await.expect("parse create response");
        let api_key = create_body["api_key"].as_str().expect("api_key exists");
        assert!(!api_key.is_empty());

        {
            let conn = lock_db(&state.db);
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM users WHERE username = ?1",
                    params!["testuser"],
                    |row| row.get(0),
                )
                .expect("query count");
            assert_eq!(count, 1);
        }

        let create_key_resp = client
            .post(format!("{}/users/{}/keys", app_url, "testuser"))
            .send()
            .await
            .expect("create api key request");
        let create_key_status = create_key_resp.status();
        let create_key_body: serde_json::Value = if create_key_status != ReqwestStatusCode::CREATED
        {
            let _body = create_key_resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<body error: {}>", e));
            panic!("unexpected create key status: {}", create_key_status);
        } else {
            create_key_resp
                .json()
                .await
                .expect("parse create key response")
        };
        assert_eq!(create_key_status, ReqwestStatusCode::CREATED);
        let additional_api_key = create_key_body["api_key"].as_str().expect("api_key exists");
        assert!(!additional_api_key.is_empty());

        let chat_resp = client
            .post(format!("{}/v1/chat/completions", app_url))
            .bearer_auth(additional_api_key)
            .json(&json!({
                "model": "gpt-4",
                "messages": [{"role": "user", "content": "hi"}],
            }))
            .send()
            .await
            .expect("chat request");
        let chat_status = chat_resp.status();
        let chat_body: serde_json::Value = if chat_status != ReqwestStatusCode::OK {
            let _ = chat_resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<body error: {}>", e));
            panic!("unexpected chat status: {}", chat_status);
        } else {
            chat_resp.json().await.expect("parse chat response")
        };
        assert_eq!(chat_status, ReqwestStatusCode::OK);
        assert_eq!(
            chat_body["choices"][0]["message"]["content"],
            "Hello from mock"
        );

        app_handle.abort();
        upstream_handle.abort();
    }
}
