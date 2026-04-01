pub mod audit;
pub mod auth;
pub mod config;
pub mod db;
pub mod errors;
pub mod handlers;
pub mod routes;
pub mod services;
#[cfg(test)]
pub(crate) mod test_utils;
pub mod upstream;

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
    }
}

pub async fn app_main_with_dir<P: AsRef<Path>>(dir: P, test_mode: bool) -> std::io::Result<()> {
    let cfg = config::load_config_from_dir(dir).unwrap_or_else(|e| {
        panic!("Failed to load config (config.toml or env): {e}");
    });
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
    app_main(false).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{AuditConfig, AuditMessage};
    use crate::config::{AppConfig, ServerConfig, SqliteConfig, UpstreamConfig};
    use crate::test_utils::with_env_lock_async;
    use actix_web::{http::StatusCode, test};
    use r2d2_sqlite::SqliteConnectionManager;
    use std::env;

    #[actix_web::test]
    async fn routes_register_health_route() {
        let app = test::init_service(App::new().configure(routes::configure_routes)).await;
        let req = test::TestRequest::get().uri("/healthz").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn create_app_middleware_handles_health_request() {
        let cfg = AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".into(),
                port: 0,
            },
            upstream: UpstreamConfig {
                base_url: "https://api.openai.com/v1".into(),
                api_key: "test".into(),
            },
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
            auth: crate::config::AuthConfig {
                invite_code: "ZW9Z".into(),
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
        let state = AppState {
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
        };

        let app = test::init_service({
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
                .app_data(web::Data::new(state))
                .configure(routes::configure_routes)
        })
        .await;
        let req = test::TestRequest::get().uri("/healthz").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn app_main_test_mode_creates_app() {
        with_env_lock_async(|| async {
            let original_value = env::var("UPSTREAM_API_KEY").ok();
            env::set_var("UPSTREAM_API_KEY", "test");
            let result = app_main(true).await;
            assert!(result.is_ok());
            match original_value {
                Some(value) => env::set_var("UPSTREAM_API_KEY", value),
                None => env::remove_var("UPSTREAM_API_KEY"),
            }
        })
        .await;
    }
}
