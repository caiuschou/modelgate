pub mod api_key_policy;
pub mod audit;
pub mod auth;
pub mod billing;
pub mod byok;
pub mod config;
pub mod db;
pub mod errors;
pub mod handlers;
pub mod jwt_session;
pub mod key_concurrency;
pub mod logging;
pub mod money;
pub mod routes;
pub mod secrets;
pub mod services;
pub mod session_auth;
#[cfg(test)]
pub(crate) mod test_utils;
pub mod upstream;
pub mod version;

use actix_cors::Cors;
use actix_web::{
    body::{BoxBody, EitherBody},
    dev::{Service, ServiceRequest, ServiceResponse},
    web, App, HttpServer,
};
use std::path::Path;
use std::sync::Arc;
use tracing::{error, info};

use crate::audit::{audit_writer_loop, ensure_storage_dirs, AuditConfig, AuditMessage};
use crate::config::AppConfig;
use crate::db::{create_db_pool, run_migrations, DbConn};

/// Actix-web defaults to 256 KiB for `web::Bytes` / `web::Json` bodies; the chat proxy must not
/// cap client payloads below upstream expectations.
const MAX_HTTP_PAYLOAD_BYTES: usize = usize::MAX;

#[derive(Clone)]
pub struct AppState {
    pub cfg: AppConfig,
    pub http: reqwest::Client,
    pub db: DbConn,
    pub auth_service: Arc<dyn services::AuthService>,
    pub audit_service: Arc<dyn services::AuditService>,
    pub user_service: Arc<dyn services::UserService>,
    pub audit_sender: tokio::sync::mpsc::Sender<AuditMessage>,
    pub audit_config: AuditConfig,
    /// Per–API key in-flight chat slots (see `key_concurrency`).
    pub key_concurrency: crate::key_concurrency::KeyConcurrencyRegistry,
}

pub fn build_state(cfg: AppConfig) -> AppState {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .expect("failed to build http client");

    let db = create_db_pool(&cfg.sqlite.path)
        .unwrap_or_else(|e| panic!("failed to create sqlite pool: {e}"));
    {
        let conn = db
            .get()
            .unwrap_or_else(|e| panic!("failed to get sqlite connection: {e}"));
        run_migrations(&conn).expect("failed to run database migrations");
    }
    ensure_storage_dirs(&cfg.audit).expect("failed to prepare audit storage dirs");
    let (audit_sender, _audit_receiver) = tokio::sync::mpsc::channel(1024);
    let service_container = services::build_service_container(db.clone());

    AppState {
        audit_config: cfg.audit.clone(),
        cfg,
        http,
        db,
        auth_service: service_container.auth,
        audit_service: service_container.audit,
        user_service: service_container.user,
        audit_sender,
        key_concurrency: std::sync::Arc::new(dashmap::DashMap::new()),
    }
}

pub async fn app_main_with_dir<P: AsRef<Path>>(dir: P, test_mode: bool) -> std::io::Result<()> {
    let cfg = config::load_config_from_dir(dir).unwrap_or_else(|e| {
        panic!("Failed to load config (config.toml or env): {e}");
    });
    if !test_mode {
        logging::init_tracing(&cfg.logging);
        info!(version = %crate::version::full_version_line(), "modelgate starting");
    }
    let state = build_state(cfg.clone());
    let (audit_sender, audit_receiver) = tokio::sync::mpsc::channel(4096);
    let writer_state = state.db.clone();
    let writer_cfg = cfg.audit.clone();
    tokio::spawn(async move {
        audit_writer_loop(audit_receiver, writer_state, writer_cfg).await;
    });
    let state = AppState {
        audit_sender,
        ..state
    };

    if test_mode {
        let _ = App::new().configure(routes::configure_routes);
        return Ok(());
    }

    let host = state.cfg.server.host.clone();
    let port = state.cfg.server.port;
    info!(%host, %port, "starting server");
    info!(
        upstream_base_url = %state.cfg.upstream.base_url,
        upstream_api_key_masked = %crate::secrets::mask_secret(&state.cfg.upstream.api_key),
        upstream_api_key_sha256 = %crate::secrets::secret_sha256_hex(&state.cfg.upstream.api_key),
        "upstream config at startup (masked key; sha256 matches `echo -n KEY | sha256sum` / local file)"
    );

    let state_clone = state.clone();
    HttpServer::new(move || {
        let cors = Cors::permissive();
        let state = state_clone.clone();

        App::new()
            .wrap(cors)
            .wrap_fn(move |req: ServiceRequest, srv| {
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
            .app_data(web::PayloadConfig::default().limit(MAX_HTTP_PAYLOAD_BYTES))
            .app_data(web::JsonConfig::default().limit(MAX_HTTP_PAYLOAD_BYTES))
            .app_data(web::Data::new(state))
            .configure(routes::configure_routes)
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}

pub async fn app_main(test_mode: bool) -> std::io::Result<()> {
    app_main_with_dir(
        std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()),
        test_mode,
    )
    .await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("{}", crate::version::full_version_line());
                return Ok(());
            }
            "--help" | "-h" => {
                println!(
                    "modelgate {}\n\nUsage: modelgate [--version|-V] [--help|-h]\n\nRuns the API server (config.toml in the working directory).\nPublic endpoints: GET /healthz, GET /version",
                    crate::version::VERSION
                );
                return Ok(());
            }
            _ => {}
        }
    }
    app_main(false).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{AuditConfig, AuditMessage};
    use crate::config::{AppConfig, BillingConfig, ServerConfig, SqliteConfig, UpstreamConfig};
    use crate::db;
    use crate::jwt_session;
    use crate::test_utils::with_env_lock_async;
    use actix_web::dev::ServiceRequest;
    use actix_web::http::header::ContentType;
    use actix_web::{http::StatusCode, test, HttpResponse};
    use bytes::Bytes;
    use r2d2_sqlite::SqliteConnectionManager;
    use std::env;

    /// Actix-web `PayloadConfig` / `JsonConfig` default cap (256 KiB).
    const DEFAULT_ACTIX_PAYLOAD_LIMIT: usize = 262_144;
    /// One byte beyond the default actix payload limit (triggers 413 without an explicit raise).
    const OVER_DEFAULT_ACTIX_BODY_LEN: usize = DEFAULT_ACTIX_PAYLOAD_LIMIT + 1;

    /// Gateway-style `App` for integration tests (`with_limits` matches production payload policy).
    macro_rules! gateway_test_app {
        ($state:expr, with_limits) => {
            App::new()
                .wrap(Cors::permissive())
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
                                Ok(res)
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
                .app_data(web::PayloadConfig::default().limit(super::MAX_HTTP_PAYLOAD_BYTES))
                .app_data(web::JsonConfig::default().limit(super::MAX_HTTP_PAYLOAD_BYTES))
                .app_data(web::Data::new($state))
                .configure(routes::configure_routes)
        };
        ($state:expr, no_limits) => {
            App::new()
                .wrap(Cors::permissive())
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
                                Ok(res)
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
                .app_data(web::Data::new($state))
                .configure(routes::configure_routes)
        };
    }

    fn build_test_app_state_with_billing(billing: BillingConfig) -> AppState {
        let cfg = AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".into(),
                port: 0,
            },
            upstream: UpstreamConfig {
                base_url: "https://api.openai.com/v1".into(),
                api_key: "test".into(),
            },
            byok: crate::config::ByokConfig::default(),
            billing,
            sqlite: SqliteConfig {
                path: ":memory:".into(),
            },
            audit: AuditConfig {
                log_dir: "./audit_logs".into(),
                retention_days: 90,
                batch_size: 50,
                flush_interval_seconds: 5,
                export_dir: "./exports".into(),
            },
            logging: crate::config::LoggingConfig::default(),
            auth: crate::config::AuthConfig {
                invite_code: "ZW9Z".into(),
                jwt_secret: "main-test-jwt-secret-min-32-chars!!!".into(),
            },
        };
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("failed to build http client");
        let manager = SqliteConnectionManager::memory();
        let db_pool = r2d2::Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("build sqlite pool");
        {
            let conn = db_pool.get().expect("get sqlite conn");
            crate::db::run_migrations(&conn).expect("run migrations");
        }
        let service_container = services::build_service_container(db_pool.clone());
        AppState {
            cfg,
            http,
            db: db_pool,
            auth_service: service_container.auth,
            audit_service: service_container.audit,
            user_service: service_container.user,
            audit_sender: tokio::sync::mpsc::channel::<AuditMessage>(16).0,
            audit_config: AuditConfig {
                log_dir: "./audit_logs".into(),
                retention_days: 90,
                batch_size: 50,
                flush_interval_seconds: 5,
                export_dir: "./exports".into(),
            },
            key_concurrency: std::sync::Arc::new(dashmap::DashMap::new()),
        }
    }

    fn build_test_app_state() -> AppState {
        build_test_app_state_with_billing(BillingConfig::default())
    }

    fn unix_now_secs() -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    async fn echo_bytes_len(body: web::Bytes) -> HttpResponse {
        HttpResponse::Ok().body(body.len().to_string())
    }

    async fn echo_json_len(body: web::Json<serde_json::Value>) -> HttpResponse {
        HttpResponse::Ok().body(body.to_string().len().to_string())
    }

    #[actix_web::test]
    async fn routes_register_health_route() {
        let app = test::init_service(App::new().configure(routes::configure_routes)).await;
        let req = test::TestRequest::get().uri("/healthz").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn actix_default_bytes_extractor_rejects_body_over_256kb() {
        let app =
            test::init_service(App::new().route("/echo", web::post().to(echo_bytes_len))).await;
        let req = test::TestRequest::post()
            .uri("/echo")
            .insert_header(ContentType::plaintext())
            .set_payload(Bytes::from(vec![b'x'; OVER_DEFAULT_ACTIX_BODY_LEN]))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[actix_web::test]
    async fn raised_payload_limit_accepts_bytes_over_actix_default() {
        let app = test::init_service(
            App::new()
                .app_data(web::PayloadConfig::default().limit(super::MAX_HTTP_PAYLOAD_BYTES))
                .route("/echo", web::post().to(echo_bytes_len)),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/echo")
            .insert_header(ContentType::plaintext())
            .set_payload(Bytes::from(vec![b'y'; OVER_DEFAULT_ACTIX_BODY_LEN]))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = test::read_body(resp).await;
        assert_eq!(
            body.as_ref(),
            OVER_DEFAULT_ACTIX_BODY_LEN.to_string().as_bytes()
        );
    }

    #[actix_web::test]
    async fn json_extractor_rejects_body_above_configured_limit() {
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(DEFAULT_ACTIX_PAYLOAD_LIMIT))
                .route("/echo", web::post().to(echo_json_len)),
        )
        .await;
        let pad = "p".repeat(OVER_DEFAULT_ACTIX_BODY_LEN);
        let json = serde_json::json!({ "pad": pad });
        let req = test::TestRequest::post()
            .uri("/echo")
            .insert_header(ContentType::json())
            .set_payload(Bytes::from(json.to_string()))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[actix_web::test]
    async fn raised_json_limit_accepts_body_over_actix_default() {
        let app = test::init_service(
            App::new()
                .app_data(web::JsonConfig::default().limit(super::MAX_HTTP_PAYLOAD_BYTES))
                .route("/echo", web::post().to(echo_json_len)),
        )
        .await;
        let pad = "q".repeat(OVER_DEFAULT_ACTIX_BODY_LEN);
        let json = serde_json::json!({ "pad": pad });
        let req = test::TestRequest::post()
            .uri("/echo")
            .insert_header(ContentType::json())
            .set_payload(Bytes::from(json.to_string()))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = test::read_body(resp).await;
        let expected_len = json.to_string().len();
        assert_eq!(body.as_ref(), expected_len.to_string().as_bytes());
    }

    #[actix_web::test]
    async fn chat_completions_without_payload_override_rejects_large_body() {
        let state = build_test_app_state();
        let app = test::init_service(gateway_test_app!(state, no_limits)).await;
        let req = test::TestRequest::post()
            .uri("/v1/chat/completions")
            .insert_header(ContentType::json())
            .set_payload(Bytes::from(vec![b'z'; OVER_DEFAULT_ACTIX_BODY_LEN]))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[actix_web::test]
    async fn chat_completions_accepts_large_body_with_gateway_payload_limits() {
        let state = build_test_app_state();
        let app = test::init_service(gateway_test_app!(state, with_limits)).await;
        let req = test::TestRequest::post()
            .uri("/v1/chat/completions")
            .insert_header(ContentType::json())
            .set_payload(Bytes::from(vec![b'z'; OVER_DEFAULT_ACTIX_BODY_LEN]))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_ne!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn list_models_requires_gateway_api_key() {
        let state = build_test_app_state();
        let app = test::init_service(gateway_test_app!(state, with_limits)).await;
        let req = test::TestRequest::get().uri("/v1/models").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_web::test]
    async fn create_app_middleware_handles_health_request() {
        let state = build_test_app_state();
        let app = test::init_service(gateway_test_app!(state, with_limits)).await;
        let req = test::TestRequest::get().uri("/healthz").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn billing_balance_ok_for_session() {
        use actix_web::http::header::AUTHORIZATION;
        let state = build_test_app_state();
        let uid = {
            let conn = state.db.get().expect("db");
            db::create_user(&conn, "bill_user", unix_now_secs()).expect("user")
        };
        let token =
            jwt_session::encode_session_jwt(&state.cfg.auth.jwt_secret, uid, "bill_user", "user")
                .expect("jwt");
        let app = test::init_service(gateway_test_app!(state, with_limits)).await;
        let req = test::TestRequest::get()
            .uri("/api/v1/me/billing/balance")
            .insert_header((AUTHORIZATION, format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let v: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(v["balance_minor"], "0");
        assert_eq!(v["currency"], "USD");
    }

    #[actix_web::test]
    async fn billing_admin_deposit_not_configured_returns_404() {
        let state = build_test_app_state();
        let app = test::init_service(gateway_test_app!(state, with_limits)).await;
        let req = test::TestRequest::post()
            .uri("/api/v1/billing/admin-deposit")
            .insert_header(ContentType::json())
            .insert_header(("Authorization", "Bearer x"))
            .set_payload(Bytes::from(
                r#"{"username":"any","amount_usd":100}"#.as_bytes().to_vec(),
            ))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn billing_admin_deposit_increases_balance() {
        use actix_web::http::header::AUTHORIZATION;
        let billing = BillingConfig {
            admin_deposit_enabled: true,
            admin_deposit_password: "admintestpwd".into(),
            min_deposit_cents: 1,
            ..Default::default()
        };
        let state = build_test_app_state_with_billing(billing);
        let uid = {
            let conn = state.db.get().expect("db");
            db::create_user(&conn, "fundme", unix_now_secs()).expect("user")
        };
        let jwt_secret = state.cfg.auth.jwt_secret.clone();
        let app = test::init_service(gateway_test_app!(state, with_limits)).await;
        let post = test::TestRequest::post()
            .uri("/api/v1/billing/admin-deposit")
            .insert_header(ContentType::json())
            .insert_header(("Authorization", "Bearer admintestpwd"))
            .set_payload(Bytes::from(
                r#"{"username":"fundme","amount_usd":2.5}"#.as_bytes().to_vec(),
            ))
            .to_request();
        let resp = test::call_service(&app, post).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let token =
            jwt_session::encode_session_jwt(&jwt_secret, uid, "fundme", "user").expect("jwt");
        let get = test::TestRequest::get()
            .uri("/api/v1/me/billing/balance")
            .insert_header((AUTHORIZATION, format!("Bearer {token}")))
            .to_request();
        let resp2 = test::call_service(&app, get).await;
        assert_eq!(resp2.status(), StatusCode::OK);
        let v: serde_json::Value = test::read_body_json(resp2).await;
        assert_ne!(v["balance_minor"], "0");
    }

    #[actix_web::test]
    async fn app_main_test_mode_creates_app() {
        use std::fs;

        with_env_lock_async(|| async {
            env::remove_var("UPSTREAM_API_KEY");
            let dir = std::env::temp_dir().join("modelgate_app_main_test_mode");
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("create temp config dir");
            let db_path = dir.join("test.db");
            let db_str = db_path.display().to_string().replace('\\', "/");
            let config_body = format!(
                r#"[upstream]
api_key = "test"
[sqlite]
path = "{db_str}"
"#
            );
            fs::write(dir.join("config.toml"), config_body).expect("write config");

            let result = app_main_with_dir(&dir, true).await;
            assert!(result.is_ok());
            let _ = fs::remove_dir_all(&dir);
        })
        .await;
    }
}
