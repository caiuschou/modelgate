use actix_web::{http::header, web, HttpRequest, HttpResponse};

use crate::audit::{
    AuditListQuery, AuditListResponse, ExportRequest, ExportResponse, ExportStatusResponse,
};
use crate::{auth, errors::ApiError, AppState};

fn auth_scope(req: &HttpRequest, state: &web::Data<AppState>) -> Result<(i64, i64), ApiError> {
    let api_key = auth::extract_bearer_token(req)
        .ok_or_else(|| ApiError::Unauthorized("Invalid or missing API key".into()))?;
    Ok(state.auth_service.get_api_key_scope(api_key)?)
}

pub async fn list_audit_logs(
    req: HttpRequest,
    state: web::Data<AppState>,
    query: web::Query<AuditListQuery>,
) -> Result<HttpResponse, ApiError> {
    let (_token_id, user_id) = auth_scope(&req, &state)?;
    let limit = query.limit.unwrap_or(100).clamp(1, 1000);
    let offset = query.offset.unwrap_or(0);

    let (data, total) = state.audit_service.list_audit_logs(&query, user_id)?;

    Ok(HttpResponse::Ok().json(AuditListResponse {
        data,
        total,
        limit,
        offset,
    }))
}

pub async fn get_audit_log(
    req: HttpRequest,
    state: web::Data<AppState>,
    request_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let (_token_id, user_id) = auth_scope(&req, &state)?;
    let record = state.audit_service.get_audit_log(&request_id, user_id)?;
    Ok(HttpResponse::Ok().json(record))
}

pub async fn export_audit_logs(
    req: HttpRequest,
    state: web::Data<AppState>,
    payload: web::Json<ExportRequest>,
) -> Result<HttpResponse, ApiError> {
    let (_token_id, user_id) = auth_scope(&req, &state)?;
    let resp: ExportResponse =
        state
            .audit_service
            .export_audit_logs(user_id, &payload, &state.audit_config.export_dir)?;
    Ok(HttpResponse::Ok().json(resp))
}

pub async fn get_export_status(
    req: HttpRequest,
    state: web::Data<AppState>,
    export_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let _ = auth_scope(&req, &state)?;
    let export_id = export_id.into_inner();
    let resp: ExportStatusResponse = state
        .audit_service
        .get_export_status(&export_id, &state.audit_config.export_dir)?;
    Ok(HttpResponse::Ok().json(resp))
}

pub async fn download_export_file(
    req: HttpRequest,
    state: web::Data<AppState>,
    export_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let _ = auth_scope(&req, &state)?;
    let export_id = export_id.into_inner();
    let file = state
        .audit_service
        .download_export_file(&export_id, &state.audit_config.export_dir)?;

    Ok(HttpResponse::Ok()
        .append_header((header::CONTENT_TYPE, file.content_type))
        .append_header((
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", file.file_name),
        ))
        .body(file.bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{body::to_bytes, http::StatusCode, test, App};
    use std::sync::Arc;

    use crate::audit::{AuditConfig, AuditListItem, AuditRecord};
    use crate::services::error::ServiceError;
    use crate::services::{AuditService, AuthService, UserService};
    use crate::{db, routes, AppState};

    struct MockAuthService;

    impl AuthService for MockAuthService {
        fn get_api_key_scope(&self, api_key: &str) -> Result<(i64, i64), ServiceError> {
            if api_key == "ok-token" {
                Ok((1, 100))
            } else {
                Err(ServiceError::Unauthorized(
                    "Invalid or missing API key".into(),
                ))
            }
        }
    }

    struct MockAuditService;

    impl AuditService for MockAuditService {
        fn list_audit_logs(
            &self,
            query: &AuditListQuery,
            _user_id: i64,
        ) -> Result<(Vec<AuditListItem>, i64), ServiceError> {
            Ok((
                vec![AuditListItem {
                    request_id: "req_1".into(),
                    user_id: Some(100),
                    token_id: Some(1),
                    channel_id: None,
                    model: Some("gpt-test".into()),
                    request_type: Some("chat".into()),
                    status_code: Some(200),
                    error_message: None,
                    prompt_tokens: Some(10),
                    completion_tokens: Some(20),
                    total_tokens: Some(30),
                    cost: Some(0.01),
                    latency_ms: Some(100),
                    app_id: Some("demo-app".into()),
                    finish_reason: Some("stop".into()),
                    created_at: query.start_time.unwrap_or(1),
                }],
                1,
            ))
        }

        fn get_audit_log(
            &self,
            request_id: &str,
            _user_id: i64,
        ) -> Result<AuditRecord, ServiceError> {
            Ok(AuditRecord {
                request_id: request_id.to_string(),
                user_id: Some(100),
                token_id: Some(1),
                channel_id: None,
                model: Some("gpt-test".into()),
                request_type: Some("chat".into()),
                request_body_path: Some("request.json".into()),
                response_body_path: Some("response.json".into()),
                status_code: Some(200),
                error_message: None,
                prompt_tokens: Some(10),
                completion_tokens: Some(20),
                total_tokens: Some(30),
                cost: Some(0.01),
                latency_ms: Some(100),
                app_id: Some("demo-app".into()),
                finish_reason: Some("stop".into()),
                metadata: None,
                created_at: 1,
            })
        }

        fn export_audit_logs(
            &self,
            _user_id: i64,
            _payload: &crate::audit::ExportRequest,
            _export_dir: &str,
        ) -> Result<crate::audit::ExportResponse, ServiceError> {
            Ok(crate::audit::ExportResponse {
                export_id: "exp_1".into(),
                status: "success".into(),
                download_url: "/api/v1/logs/export/exp_1/download".into(),
            })
        }

        fn get_export_status(
            &self,
            export_id: &str,
            _export_dir: &str,
        ) -> Result<crate::audit::ExportStatusResponse, ServiceError> {
            Ok(crate::audit::ExportStatusResponse {
                export_id: export_id.to_string(),
                status: "success".into(),
            })
        }

        fn download_export_file(
            &self,
            _export_id: &str,
            _export_dir: &str,
        ) -> Result<crate::services::audit::ExportFileData, ServiceError> {
            Ok(crate::services::audit::ExportFileData {
                bytes: b"csv,data\n1,2\n".to_vec(),
                content_type: "text/csv; charset=utf-8".into(),
                file_name: "exp_1.csv".into(),
            })
        }
    }

    struct MockUserService;

    impl UserService for MockUserService {
        fn create_user_with_api_key(
            &self,
            _username: &str,
            _api_key: &str,
            _created_at: u64,
        ) -> Result<(), ServiceError> {
            Ok(())
        }

        fn create_api_key_for_user(
            &self,
            _username: &str,
            _api_key: &str,
            _created_at: u64,
        ) -> Result<(), ServiceError> {
            Ok(())
        }

        fn register_user_with_password_and_api_key(
            &self,
            _username: &str,
            _password_hash: &str,
            _api_key: &str,
            _created_at: u64,
        ) -> Result<(), ServiceError> {
            Ok(())
        }

        fn get_user_login_credentials(
            &self,
            _username: &str,
        ) -> Result<Option<(i64, Option<String>)>, ServiceError> {
            Ok(None)
        }

        fn get_first_api_key_for_user(
            &self,
            _user_id: i64,
        ) -> Result<Option<String>, ServiceError> {
            Ok(None)
        }

        fn create_api_key_for_user_id(
            &self,
            _user_id: i64,
            _api_key: &str,
            _created_at: u64,
        ) -> Result<(), ServiceError> {
            Ok(())
        }

        fn list_my_api_keys(
            &self,
            _user_id: i64,
        ) -> Result<Vec<crate::services::repository::ApiKeySummary>, ServiceError> {
            Ok(Vec::new())
        }

        fn create_my_api_key(
            &self,
            _user_id: i64,
            _created_at: u64,
        ) -> Result<(i64, String, u64), ServiceError> {
            Ok((1, "sk-test".into(), _created_at))
        }

        fn revoke_my_api_key(&self, _user_id: i64, _key_id: i64) -> Result<(), ServiceError> {
            Ok(())
        }
    }

    fn build_test_state() -> AppState {
        let cfg = crate::config::AppConfig {
            server: crate::config::ServerConfig {
                host: "127.0.0.1".into(),
                port: 0,
            },
            upstream: crate::config::UpstreamConfig {
                base_url: "https://api.openai.com/v1".into(),
                api_key: "test".into(),
            },
            sqlite: crate::config::SqliteConfig {
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
        let db_pool = db::create_db_pool(":memory:").expect("create db pool");
        {
            let conn = db_pool.get().expect("get sqlite connection");
            db::run_migrations(&conn).expect("run migrations");
        }
        AppState {
            cfg: cfg.clone(),
            http: reqwest::Client::new(),
            db: db_pool,
            auth_service: Arc::new(MockAuthService),
            audit_service: Arc::new(MockAuditService),
            user_service: Arc::new(MockUserService),
            audit_sender: tokio::sync::mpsc::channel(4).0,
            audit_config: cfg.audit,
        }
    }

    #[actix_web::test]
    async fn list_audit_logs_route_works_with_service() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(build_test_state()))
                .configure(routes::configure_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/api/v1/logs/request?limit=10")
            .insert_header(("Authorization", "Bearer ok-token"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body()).await.expect("read body");
        let body: serde_json::Value = serde_json::from_slice(&bytes).expect("parse json");
        assert_eq!(body["total"], 1);
        assert_eq!(body["data"][0]["request_id"], "req_1");
    }

    #[actix_web::test]
    async fn export_download_route_returns_file_headers_and_body() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(build_test_state()))
                .configure(routes::configure_routes),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/logs/export/exp_1/download")
            .insert_header(("Authorization", "Bearer ok-token"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let headers = resp.headers();
        assert_eq!(
            headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or(""),
            "text/csv; charset=utf-8"
        );
        assert!(headers
            .get("content-disposition")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .contains("exp_1.csv"));
        let bytes = to_bytes(resp.into_body()).await.expect("read body");
        assert_eq!(bytes.as_ref(), b"csv,data\n1,2\n");
    }

    #[actix_web::test]
    async fn unauthorized_when_missing_token() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(build_test_state()))
                .configure(routes::configure_routes),
        )
        .await;
        let req = test::TestRequest::get()
            .uri("/api/v1/logs/request")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
