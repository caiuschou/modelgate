use actix_web::web;

use crate::handlers::{self, audit, proxy, session, team, user};

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
            "/api/v1/analytics",
            web::get().to(audit::get_audit_analytics),
        )
        .route(
            "/api/v1/logs/request",
            web::get().to(audit::list_audit_logs),
        )
        .route(
            "/api/v1/logs/request/{request_id}/body",
            web::get().to(audit::get_audit_log_body),
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
        .route(
            "/api/v1/me/api-keys",
            web::get().to(handlers::api_keys::list_my_api_keys),
        )
        .route(
            "/api/v1/me/api-keys",
            web::post().to(handlers::api_keys::create_my_api_key),
        )
        .route(
            "/api/v1/me/api-keys/{key_id}",
            web::get().to(handlers::api_keys::get_my_api_key),
        )
        .route(
            "/api/v1/me/api-keys/{key_id}",
            web::patch().to(handlers::api_keys::patch_my_api_key),
        )
        .route(
            "/api/v1/me/api-keys/{key_id}/revoke",
            web::post().to(handlers::api_keys::revoke_my_api_key),
        )
        .route("/api/v1/teams", web::get().to(team::list_my_teams))
        .route("/api/v1/teams", web::post().to(team::create_team))
        .route(
            "/api/v1/teams/{id}/members",
            web::get().to(team::list_team_members),
        )
        .route(
            "/api/v1/teams/{id}/members/register-on-behalf",
            web::post().to(team::register_member_on_behalf),
        )
        .route(
            "/api/v1/teams/{id}/invitations",
            web::post().to(team::create_invitation),
        )
        .route(
            "/api/v1/teams/{team_id}/invitations/{invitation_id}",
            web::delete().to(team::delete_invitation),
        )
        .route(
            "/api/v1/teams/{id}/members/{user_id}",
            web::patch().to(team::patch_team_member),
        )
        .route(
            "/api/v1/teams/{id}/members/{user_id}",
            web::delete().to(team::remove_team_member),
        )
        .route("/api/v1/teams/{id}", web::get().to(team::get_team))
        .route("/api/v1/teams/{id}", web::patch().to(team::patch_team))
        .route("/api/v1/teams/{id}", web::delete().to(team::delete_team))
        .route(
            "/api/v1/invitations/accept",
            web::post().to(team::accept_invitation),
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
