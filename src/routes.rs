use actix_web::web;

use crate::handlers;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/healthz", web::get().to(handlers::health))
        .route("/users", web::post().to(handlers::create_user))
        .route(
            "/users/{username}/keys",
            web::post().to(handlers::create_user_api_key),
        )
        .route(
            "/v1/chat/completions",
            web::post().to(handlers::chat_completions),
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
