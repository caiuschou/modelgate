pub mod auth;
pub mod config;
pub mod db;
pub mod errors;
pub mod handlers;
pub mod routes;
pub mod upstream;
#[cfg(test)]
pub(crate) mod test_utils;

use actix_cors::Cors;
use actix_web::{body::{BoxBody, EitherBody}, dev::{Service, ServiceRequest, ServiceResponse}, web, App, HttpServer};
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{error, info};

use crate::config::AppConfig;
use crate::db::{run_migrations, DbConn};

#[derive(Debug, Clone)]
pub struct AppState {
    pub cfg: AppConfig,
    pub http: reqwest::Client,
    pub db: DbConn,
}

pub fn build_state(cfg: AppConfig) -> AppState {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .expect("failed to build http client");

    let db = Connection::open(&cfg.sqlite.path)
        .unwrap_or_else(|e| panic!("failed to open sqlite database: {e}"));
    run_migrations(&db).expect("failed to run database migrations");

    AppState {
        cfg,
        http,
        db: Arc::new(Mutex::new(db)),
    }
}



pub async fn app_main_with_dir<P: AsRef<Path>>(dir: P, test_mode: bool) -> std::io::Result<()> {
    let cfg = config::load_config_from_dir(dir).unwrap_or_else(|e| {
        panic!("Failed to load config (config.toml or env): {e}");
    });
    let state = build_state(cfg);

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
    app_main_with_dir(std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()), test_mode).await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    app_main(false).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::StatusCode, test};
    use crate::config::{AppConfig, ServerConfig, SqliteConfig, UpstreamConfig};
    use crate::test_utils::with_env_lock_async;
    use rusqlite::Connection;
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
        };
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("failed to build http client");
        let conn = Connection::open_in_memory().expect("open in-memory sqlite");
        crate::db::run_migrations(&conn).expect("run migrations");
        let state = AppState {
            cfg,
            http,
            db: Arc::new(Mutex::new(conn)),
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
        }).await;
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
