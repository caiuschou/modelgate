use actix_web::web;

use crate::handlers::{self, audit, proxy, session, user};

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/healthz", web::get().to(handlers::health))
        .route("/api/v1/auth/register", web::post().to(session::register))
        .route("/api/v1/auth/login", web::post().to(session::login))
        .route("/users", web::post().to(user::create_user))
        .route(
            "/users/{username}/keys",
            web::post().to(user::create_user_api_key),
        )
        .route(
            "/v1/chat/completions",
            web::post().to(proxy::chat_completions),
        )
        .route(
            "/api/v1/logs/request",
            web::get().to(audit::list_audit_logs),
        )
        .route(
            "/api/v1/logs/request/{request_id}",
            web::get().to(audit::get_audit_log),
        )
        .route(
            "/api/v1/logs/export",
            web::post().to(audit::export_audit_logs),
        )
        .route(
            "/api/v1/logs/export/{export_id}",
            web::get().to(audit::get_export_status),
        )
        .route(
            "/api/v1/logs/export/{export_id}/download",
            web::get().to(audit::download_export_file),
        )
        .default_service(web::route().to(handlers::not_found));
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn health_route_is_registered() {
        let app = test::init_service(App::new().configure(configure_routes)).await;
        let req = test::TestRequest::get().uri("/healthz").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn default_route_returns_404() {
        let app = test::init_service(App::new().configure(configure_routes)).await;
        let req = test::TestRequest::get().uri("/not-found").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
