use actix_web::{http::StatusCode as ActixStatusCode, web, HttpRequest, HttpResponse};
use futures_util::StreamExt;
use reqwest::header as reqwest_header;
use rusqlite::{params, ErrorCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::error;
use uuid::Uuid;

use crate::{auth, db, errors::ApiError, upstream, AppState};

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
}

#[derive(Serialize)]
pub struct CreateUserResponse {
    pub username: String,
    pub api_key: String,
    pub created_at: u64,
}

fn create_api_key() -> String {
    Uuid::new_v4().to_string()
}

pub async fn health() -> HttpResponse {
    HttpResponse::Ok().body("ok")
}

pub async fn not_found(req: HttpRequest) -> HttpResponse {
    HttpResponse::NotFound().json(json!({
        "error": {
            "message": "Not Found",
            "code": 404,
            "path": req.path(),
            "method": req.method().as_str()
        }
    }))
}

pub async fn create_user(
    state: web::Data<AppState>,
    req: web::Json<CreateUserRequest>,
) -> Result<HttpResponse, ApiError> {
    let username = req.username.trim();
    if username.is_empty() {
        return Err(ApiError::BadRequest("username is required".into()));
    }

    let api_key = create_api_key();
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut conn = db::lock_db(&state.db);
    let tx = conn
        .transaction()
        .map_err(|err| {
            error!(error = %err, "failed to begin transaction");
            ApiError::InternalError("Failed to create user".into())
        })?;

    if let Err(err) = tx.execute(
        "INSERT INTO users (username, created_at) VALUES (?1, ?2)",
        rusqlite::params![username, created_at],
    ) {
        if let rusqlite::Error::SqliteFailure(err_code, _) = &err {
            if err_code.code == ErrorCode::ConstraintViolation {
                return Err(ApiError::Conflict("username already exists".into()));
            }
        }
        error!(error = %err, "failed to create user");
        return Err(ApiError::InternalError("Failed to create user".into()));
    }

    let user_id = tx.last_insert_rowid();
    if let Err(err) = tx.execute(
        "INSERT INTO api_keys (user_id, api_key, created_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![user_id, api_key, created_at],
    ) {
        error!(error = %err, "failed to create user api key");
        return Err(ApiError::InternalError("Failed to create user".into()));
    }

    tx.commit().map_err(|err| {
        error!(error = %err, "failed to commit transaction");
        ApiError::InternalError("Failed to create user".into())
    })?;

    Ok(HttpResponse::Created().json(CreateUserResponse {
        username: username.to_string(),
        api_key,
        created_at,
    }))
}

pub async fn create_user_api_key(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let username = path.into_inner();
    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let api_key = create_api_key();

    let conn = db::lock_db(&state.db);
    let user_id = db::find_user_id(&conn, &username)
        .map_err(|_| ApiError::NotFound("user not found".into()))?;

    conn.execute(
        "INSERT INTO api_keys (user_id, api_key, created_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![user_id, api_key, created_at],
    )
    .map_err(|err| {
        error!(error = %err, "failed to create api key for user");
        ApiError::InternalError("Failed to create api key".into())
    })?;

    Ok(HttpResponse::Created().json(CreateUserResponse {
        username,
        api_key,
        created_at,
    }))
}

pub async fn chat_completions(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Bytes,
) -> Result<HttpResponse, ApiError> {
    let api_key = auth::extract_bearer_token(&req)
        .ok_or_else(|| ApiError::Unauthorized("Invalid or missing API key".into()))?;

    let db = db::lock_db(&state.db);
    if !auth::validate_api_key(&db, api_key) {
        return Err(ApiError::Unauthorized("Invalid or missing API key".into()));
    }

    let upstream_url = upstream::build_chat_completions_url(&state.cfg.upstream.base_url);
    let mut req_builder = state
        .http
        .post(upstream_url)
        .header(
            reqwest_header::AUTHORIZATION,
            format!("Bearer {}", state.cfg.upstream.api_key),
        )
        .header(reqwest_header::CONTENT_TYPE, "application/json")
        .body(body.clone());

    if let Ok(org) = std::env::var("OPENAI_ORGANIZATION") {
        req_builder = req_builder.header("OpenAI-Organization", org);
    }
    if let Ok(project) = std::env::var("OPENAI_PROJECT") {
        req_builder = req_builder.header("OpenAI-Project", project);
    }

    let upstream_resp = req_builder.send().await.map_err(|e| {
        error!(error = %e, "upstream request failed");
        ApiError::InternalError("Upstream request failed".into())
    })?;

    let status = ActixStatusCode::from_u16(upstream_resp.status().as_u16())
        .unwrap_or(ActixStatusCode::BAD_GATEWAY);
    let is_stream = upstream::is_stream_request(&body);

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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_cors::Cors;
    use actix_web::body::{to_bytes, BoxBody, EitherBody};
    use actix_web::dev::{Service, ServiceRequest, ServiceResponse};
    use actix_web::http::{header, StatusCode as ActixStatusCode};
    use actix_web::{test::TestRequest, web, App, HttpRequest, HttpResponse, HttpServer, ResponseError};
    use crate::{config, AppConfig, AppState};
    use crate::test_utils::with_env_lock_async;
    use reqwest::StatusCode as ReqwestStatusCode;
    use rusqlite::Connection;
    use serde_json::json;
    use std::env;
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use tracing::info;

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
        db::run_migrations(&conn).expect("run migrations");

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

        let resp = create_user(state.clone(), req).await.unwrap();
        assert_eq!(resp.status(), ActixStatusCode::CREATED);

        let conn = db::lock_db(&state.db);
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
    async fn create_user_empty_username_returns_bad_request() {
        let state = web::Data::new(test_state());
        let req = web::Json(CreateUserRequest {
            username: "   ".into(),
        });

        let err = create_user(state.clone(), req).await.unwrap_err();
        assert_eq!(err.error_response().status(), ActixStatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_user_api_key_for_missing_user_returns_not_found() {
        let state = web::Data::new(test_state());
        let path = web::Path::from("missing".to_string());
        let err = create_user_api_key(state.clone(), path).await.unwrap_err();
        assert_eq!(err.error_response().status(), ActixStatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn chat_completions_unauthorized_without_api_key() {
        let state = web::Data::new(test_state());
        let req = TestRequest::post()
            .uri("/v1/chat/completions")
            .to_http_request();
        let body = web::Bytes::from_static(b"{\"model\": \"gpt-4\"}");

        let err = chat_completions(req, state, body).await.unwrap_err();
        assert_eq!(err.error_response().status(), ActixStatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn chat_completions_forwards_openai_headers() {
        with_env_lock_async(|| async {
            let upstream_listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind upstream");
            let upstream_addr = upstream_listener.local_addr().expect("local addr");
            let upstream_server = HttpServer::new(|| {
                App::new().route(
                    "/v1/chat/completions",
                    web::post().to(|req: HttpRequest| async move {
                        assert_eq!(req.headers().get("OpenAI-Organization").unwrap(), "org-test");
                        assert_eq!(req.headers().get("OpenAI-Project").unwrap(), "proj-test");
                        HttpResponse::Ok().json(json!({
                            "id": "chatcmpl-123",
                            "object": "chat.completion",
                            "choices": [{"message": {"role": "assistant", "content": "ok"}}]
                        }))
                    }),
                )
            })
            .listen(upstream_listener)
            .expect("listen upstream")
            .run();
            let upstream_handle = tokio::spawn(upstream_server);

            let conn = Connection::open_in_memory().expect("open in-memory sqlite");
            db::run_migrations(&conn).expect("run migrations");
            conn.execute(
                "INSERT INTO users (username, created_at) VALUES (?1, ?2)",
                params!["testuser", 1],
            )
            .expect("insert user");
            conn.execute(
                "INSERT INTO api_keys (user_id, api_key, created_at) VALUES (?1, ?2, ?3)",
                params![1, "valid-key", 1],
            )
            .expect("insert api key");

            let state = web::Data::new(AppState {
                cfg: AppConfig {
                    server: config::ServerConfig {
                        host: "127.0.0.1".into(),
                        port: 0,
                    },
                    upstream: config::UpstreamConfig {
                        base_url: format!("http://{}", upstream_addr),
                        api_key: "test".into(),
                    },
                    sqlite: config::SqliteConfig {
                        path: ":memory:".into(),
                    },
                },
                http: reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .build()
                    .expect("failed to build http client"),
                db: Arc::new(Mutex::new(conn)),
            });

            let original_org = env::var("OPENAI_ORGANIZATION").ok();
            let original_project = env::var("OPENAI_PROJECT").ok();
            env::set_var("OPENAI_ORGANIZATION", "org-test");
            env::set_var("OPENAI_PROJECT", "proj-test");

            let req = TestRequest::post()
                .uri("/v1/chat/completions")
                .insert_header((header::AUTHORIZATION, "Bearer valid-key"))
                .to_http_request();
            let body = web::Bytes::from_static(b"{\"model\": \"gpt-4\", \"messages\": [{\"role\": \"user\", \"content\": \"hi\"}]} ");

            let resp = chat_completions(req, state, body).await.unwrap();
            assert_eq!(resp.status(), ActixStatusCode::OK);

            if let Some(value) = original_org {
                env::set_var("OPENAI_ORGANIZATION", value);
            } else {
                env::remove_var("OPENAI_ORGANIZATION");
            }
            if let Some(value) = original_project {
                env::set_var("OPENAI_PROJECT", value);
            } else {
                env::remove_var("OPENAI_PROJECT");
            }

            upstream_handle.abort();
        })
        .await;
    }

    #[tokio::test]
    async fn chat_completions_upstream_request_failure_returns_internal_error() {
        let mut state = test_state();
        state.cfg.upstream.base_url = "http://127.0.0.1:1".into();

        {
            let conn = db::lock_db(&state.db);
            conn.execute(
                "INSERT INTO users (username, created_at) VALUES (?1, ?2)",
                params!["testuser", 1],
            )
            .expect("insert user");
            conn.execute(
                "INSERT INTO api_keys (user_id, api_key, created_at) VALUES (?1, ?2, ?3)",
                params![1, "valid-key", 1],
            )
            .expect("insert api key");
        }

        let state = web::Data::new(state);
        let req = TestRequest::post()
            .uri("/v1/chat/completions")
            .insert_header((header::AUTHORIZATION, "Bearer valid-key"))
            .to_http_request();
        let body = web::Bytes::from_static(b"{\"model\": \"gpt-4\", \"messages\": [{\"role\": \"user\", \"content\": \"hi\"}]} ");

        let err = chat_completions(req, state, body).await.unwrap_err();
        assert_eq!(err.error_response().status(), ActixStatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn chat_completions_streaming_response_returns_stream() {
        let upstream_listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind upstream");
        let upstream_addr = upstream_listener.local_addr().expect("local addr");
        let upstream_server = HttpServer::new(|| {
            App::new().route(
                "/v1/chat/completions",
                web::post().to(|| async {
                    HttpResponse::Ok().body("streaming response")
                }),
            )
        })
        .listen(upstream_listener)
        .expect("listen upstream")
        .run();
        let upstream_handle = tokio::spawn(upstream_server);

        let mut state = test_state();
        state.cfg.upstream.base_url = format!("http://{}", upstream_addr);
        {
            let conn = db::lock_db(&state.db);
            conn.execute(
                "INSERT INTO users (username, created_at) VALUES (?1, ?2)",
                params!["testuser", 1],
            )
            .expect("insert user");
            conn.execute(
                "INSERT INTO api_keys (user_id, api_key, created_at) VALUES (?1, ?2, ?3)",
                params![1, "valid-key", 1],
            )
            .expect("insert api key");
        }

        let state = web::Data::new(state);
        let req = TestRequest::post()
            .uri("/v1/chat/completions")
            .insert_header((header::AUTHORIZATION, "Bearer valid-key"))
            .to_http_request();
        let body = web::Bytes::from_static(b"{\"stream\": true, \"model\": \"gpt-4\"}");

        let resp = chat_completions(req, state, body).await.unwrap();
        assert_eq!(resp.status(), ActixStatusCode::OK);
        let bytes = to_bytes(resp.into_body()).await.expect("body bytes");
        assert_eq!(bytes, "streaming response");

        upstream_handle.abort();
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

        let resp1 = create_user(state.clone(), req1).await.unwrap();
        assert_eq!(resp1.status(), ActixStatusCode::CREATED);

        let resp2 = create_user(state, req2).await;
        assert!(resp2.is_err());
        let resp2 = resp2.err().unwrap().error_response();
        assert_eq!(resp2.status(), ActixStatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn create_additional_api_key_for_existing_user() {
        let state = web::Data::new(test_state());
        let req = web::Json(CreateUserRequest {
            username: "charlie".into(),
        });
        let resp = create_user(state.clone(), req).await.unwrap();
        assert_eq!(resp.status(), ActixStatusCode::CREATED);

        let path = web::Path::from("charlie".to_string());
        let create_key_resp = create_user_api_key(state.clone(), path).await.unwrap();
        assert_eq!(create_key_resp.status(), ActixStatusCode::CREATED);

        let conn = db::lock_db(&state.db);
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
        db::run_migrations(&conn).expect("run migrations");

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
                .route("/users/{username}/keys", web::post().to(create_user_api_key))
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
            let conn = db::lock_db(&state.db);
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
