use actix_web::{http::header, web, HttpRequest, HttpResponse};
use serde::Deserialize;
use tracing::debug;

use crate::audit::{
    AuditAnalyticsParams, AuditListQuery, AuditListResponse, ExportRequest, ExportResponse,
    ExportStatusResponse,
};
use crate::db::AuditConsoleScope;
use crate::{errors::ApiError, session_auth, AppState};

fn auth_scope(
    req: &HttpRequest,
    state: &web::Data<AppState>,
) -> Result<(Option<i64>, i64), ApiError> {
    let s = session_auth::resolve_console_session(req, state)?;
    Ok((s.api_key_id, s.user_id))
}

pub async fn get_audit_analytics(
    req: HttpRequest,
    state: web::Data<AppState>,
    query: web::Query<AuditAnalyticsParams>,
) -> Result<HttpResponse, ApiError> {
    let (_, user_id) = auth_scope(&req, &state)?;
    let p = query.into_inner();
    let scope = if p.combined == Some(true) && session_auth::parse_x_team_id(&req).is_none() {
        AuditConsoleScope::UserOwnedTraffic(user_id)
    } else {
        session_auth::audit_scope_for_request(&req, &state, user_id)?
    };
    debug!(
        target: "audit_analytics",
        user_id,
        ?scope,
        combined = p.combined,
        start_time = p.start_time,
        end_time = p.end_time,
        model = p.model.as_deref(),
        token_id = p.token_id,
        app_id = p.app_id.as_deref(),
        thread_id = p.thread_id.as_deref(),
        "GET /api/v1/analytics"
    );
    let list_query = AuditListQuery {
        start_time: p.start_time,
        end_time: p.end_time,
        user_id: None,
        token_id: p.token_id,
        channel_id: None,
        model: p.model,
        status_code: None,
        keyword: None,
        app_id: p.app_id,
        thread_id: p.thread_id,
        finish_reason: None,
        min_prompt_tokens: None,
        max_prompt_tokens: None,
        min_completion_tokens: None,
        max_completion_tokens: None,
        limit: None,
        offset: None,
    };
    let resp = state
        .audit_service
        .get_audit_analytics(&list_query, scope)?;
    Ok(HttpResponse::Ok().json(resp))
}

pub async fn list_audit_logs(
    req: HttpRequest,
    state: web::Data<AppState>,
    query: web::Query<AuditListQuery>,
) -> Result<HttpResponse, ApiError> {
    let (_, user_id) = auth_scope(&req, &state)?;
    let scope = session_auth::audit_scope_for_request(&req, &state, user_id)?;
    let limit = query.limit.unwrap_or(100).clamp(1, 1000);
    let offset = query.offset.unwrap_or(0);

    let (data, total) = state.audit_service.list_audit_logs(&query, scope)?;

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
    let (_, user_id) = auth_scope(&req, &state)?;
    let record = state.audit_service.get_audit_log(&request_id, user_id)?;
    Ok(HttpResponse::Ok().json(record))
}

#[derive(Debug, Deserialize)]
pub struct AuditBodyQuery {
    pub part: String,
}

pub async fn get_audit_log_body(
    req: HttpRequest,
    state: web::Data<AppState>,
    request_id: web::Path<String>,
    query: web::Query<AuditBodyQuery>,
) -> Result<HttpResponse, ApiError> {
    let part = query.part.to_ascii_lowercase();
    if part != "request" && part != "response" {
        return Err(ApiError::BadRequest(
            "query parameter part must be \"request\" or \"response\"".into(),
        ));
    }
    let (_, user_id) = auth_scope(&req, &state)?;
    let request_id = request_id.into_inner();
    let record = state.audit_service.get_audit_log(&request_id, user_id)?;
    let stored_path = match part.as_str() {
        "request" => record.request_body_path.as_deref(),
        "response" => record.response_body_path.as_deref(),
        _ => unreachable!(),
    };
    let stored_path = stored_path.ok_or_else(|| {
        ApiError::NotFound(
            "No stored body for this entry (e.g. streaming responses are not saved)".into(),
        )
    })?;
    let bytes = crate::audit::read_audit_body_bytes(&state.audit_config.log_dir, stored_path)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                ApiError::NotFound("Audit body file is missing or was removed".into())
            }
            std::io::ErrorKind::PermissionDenied => {
                ApiError::Forbidden("Invalid audit body path".into())
            }
            std::io::ErrorKind::InvalidInput => {
                ApiError::BadRequest("Invalid audit body path".into())
            }
            _ => ApiError::InternalError(format!("Failed to read audit body: {e}")),
        })?;

    let content_type = if serde_json::from_slice::<serde_json::Value>(&bytes).is_ok() {
        header::ContentType::json().to_string()
    } else if std::str::from_utf8(&bytes).is_ok() {
        "text/plain; charset=utf-8".to_string()
    } else {
        "application/octet-stream".to_string()
    };

    Ok(HttpResponse::Ok()
        .append_header((header::CONTENT_TYPE, content_type))
        .body(bytes))
}

pub async fn export_audit_logs(
    req: HttpRequest,
    state: web::Data<AppState>,
    payload: web::Json<ExportRequest>,
) -> Result<HttpResponse, ApiError> {
    let (_, user_id) = auth_scope(&req, &state)?;
    let scope = session_auth::audit_scope_for_request(&req, &state, user_id)?;
    let resp: ExportResponse =
        state
            .audit_service
            .export_audit_logs(scope, &payload, &state.audit_config.export_dir)?;
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
            if api_key == "sk-or-v1-testok" {
                Ok((1, 100))
            } else {
                Err(ServiceError::Unauthorized(
                    "Invalid or missing API key".into(),
                ))
            }
        }

        fn get_api_key_auth(
            &self,
            api_key: &str,
        ) -> Result<crate::db::ApiKeyAuthRow, ServiceError> {
            if api_key == "ok-token" {
                Ok(crate::db::ApiKeyAuthRow {
                    id: 1,
                    user_id: 100,
                    model_allowlist: None,
                    ip_allowlist: None,
                    quota_monthly_tokens: None,
                    quota_used_tokens: 0,
                    quota_period_start: None,
                    team_id: None,
                    default_byok_profile_id: None,
                    max_concurrent_requests: None,
                })
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
            _scope: crate::db::AuditConsoleScope,
        ) -> Result<(Vec<AuditListItem>, i64), ServiceError> {
            Ok((
                vec![AuditListItem {
                    request_id: "req_1".into(),
                    user_id: Some(100),
                    team_id: None,
                    token_id: Some(1),
                    channel_id: None,
                    model: Some("gpt-test".into()),
                    request_type: Some("chat".into()),
                    status_code: Some(200),
                    error_message: None,
                    prompt_tokens: Some(10),
                    completion_tokens: Some(20),
                    cached_prompt_tokens: None,
                    total_tokens: Some(30),
                    cost: Some(0.01),
                    latency_ms: Some(100),
                    app_id: Some("demo-app".into()),
                    thread_id: None,
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
                request_body_path: Some(format!("0/{request_id}-request.json")),
                response_body_path: Some(format!("0/{request_id}-response.json")),
                status_code: Some(200),
                error_message: None,
                prompt_tokens: Some(10),
                completion_tokens: Some(20),
                cached_prompt_tokens: None,
                total_tokens: Some(30),
                cost: Some(0.01),
                latency_ms: Some(100),
                app_id: Some("demo-app".into()),
                thread_id: None,
                finish_reason: Some("stop".into()),
                metadata: None,
                created_at: 1,
                team_id: None,
            })
        }

        fn export_audit_logs(
            &self,
            _scope: crate::db::AuditConsoleScope,
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

        fn get_audit_analytics(
            &self,
            _query: &AuditListQuery,
            _scope: crate::db::AuditConsoleScope,
        ) -> Result<crate::audit::AuditAnalyticsResponse, ServiceError> {
            Ok(crate::audit::AuditAnalyticsResponse {
                summary: crate::audit::AuditAnalyticsSummary {
                    total_requests: 1,
                    success_requests: 1,
                    total_tokens: 30,
                    total_cost: 0.01,
                    avg_latency_ms: Some(100.0),
                },
                bucket_seconds: 3600,
                series: vec![crate::audit::AuditAnalyticsTimeBucket {
                    bucket_start: 0,
                    request_count: 1,
                    total_tokens: 30,
                    total_cost: 0.01,
                    prompt_tokens: 10,
                    completion_tokens: 20,
                    cached_prompt_tokens: 0,
                }],
                by_model: vec![crate::audit::AuditAnalyticsModelSlice {
                    model: "gpt-test".into(),
                    request_count: 1,
                    total_tokens: 30,
                }],
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

        fn change_my_password(
            &self,
            _user_id: i64,
            _new_password: &str,
        ) -> Result<(), ServiceError> {
            Ok(())
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
            _team_id: Option<i64>,
        ) -> Result<Vec<crate::services::repository::ApiKeySummary>, ServiceError> {
            Ok(Vec::new())
        }

        fn create_my_api_key(
            &self,
            _user_id: i64,
            created_at: u64,
            _input: crate::services::user::CreateMyApiKeyInput,
        ) -> Result<(i64, String, u64), ServiceError> {
            Ok((1, "sk-test".into(), created_at))
        }

        fn get_my_api_key(
            &self,
            _user_id: i64,
            _key_id: i64,
        ) -> Result<crate::services::repository::ApiKeySummary, ServiceError> {
            Err(ServiceError::NotFound("api key not found".into()))
        }

        fn update_my_api_key(
            &self,
            _user_id: i64,
            _key_id: i64,
            _patch: crate::db::ApiKeyPatchDb,
        ) -> Result<(), ServiceError> {
            Ok(())
        }

        fn revoke_my_api_key(&self, _user_id: i64, _key_id: i64) -> Result<(), ServiceError> {
            Ok(())
        }

        fn touch_api_key_last_used(&self, _key_id: i64, _now: i64) -> Result<(), ServiceError> {
            Ok(())
        }

        fn ensure_monthly_quota(&self, _key_id: i64, _now: i64) -> Result<(), ServiceError> {
            Ok(())
        }

        fn ensure_monthly_spend_quota(&self, _key_id: i64, _now: i64) -> Result<(), ServiceError> {
            Ok(())
        }

        fn increment_quota_tokens(&self, _key_id: i64, _delta: i64) -> Result<(), ServiceError> {
            Ok(())
        }
    }

    fn build_test_state() -> AppState {
        let audit_root = std::env::temp_dir().join(format!(
            "modelgate_audit_test_{}_{}",
            std::process::id(),
            crate::audit::now_unix_millis()
        ));
        std::fs::create_dir_all(audit_root.join("0")).expect("audit test subdir");
        std::fs::write(
            audit_root.join("0").join("req_1-request.json"),
            br#"{"model":"gpt-test"}"#,
        )
        .expect("write req body fixture");
        std::fs::write(
            audit_root.join("0").join("req_1-response.json"),
            br#"{"id":"chat"}"#,
        )
        .expect("write resp body fixture");
        let log_dir = audit_root.to_string_lossy().into_owned();

        let cfg = crate::config::AppConfig {
            server: crate::config::ServerConfig {
                host: "127.0.0.1".into(),
                port: 0,
            },
            upstream: crate::config::UpstreamConfig {
                base_url: "https://api.openai.com/v1".into(),
                api_key: "test".into(),
            },
            byok: crate::config::ByokConfig::default(),
            billing: crate::config::BillingConfig::default(),
            sqlite: crate::config::SqliteConfig {
                path: ":memory:".into(),
            },
            audit: AuditConfig {
                log_dir,
                retention_days: 90,
                batch_size: 50,
                flush_interval_seconds: 5,
                export_dir: "./exports".into(),
            },
            logging: crate::config::LoggingConfig::default(),
            auth: crate::config::AuthConfig {
                invite_code: "ZW9Z".into(),
                jwt_secret: "audit-test-jwt-secret-min-32-chars!!".into(),
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
            key_concurrency: std::sync::Arc::new(dashmap::DashMap::new()),
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
            .insert_header(("Authorization", "Bearer sk-or-v1-testok"))
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
            .insert_header(("Authorization", "Bearer sk-or-v1-testok"))
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
    async fn get_audit_log_body_returns_request_file() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(build_test_state()))
                .configure(routes::configure_routes),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/v1/logs/request/req_1/body?part=request")
            .insert_header(("Authorization", "Bearer sk-or-v1-testok"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body()).await.expect("read body");
        let body: serde_json::Value = serde_json::from_slice(&bytes).expect("parse json");
        assert_eq!(body["model"], "gpt-test");
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
