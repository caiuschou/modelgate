pub mod audit;
pub mod proxy;
pub mod user;

use actix_web::{HttpRequest, HttpResponse};

pub async fn health() -> HttpResponse {
    HttpResponse::Ok().body("ok")
}

pub async fn not_found(req: HttpRequest) -> HttpResponse {
    use serde_json::json;
    HttpResponse::NotFound().json(json!({
        "error": {
            "message": "Not Found",
            "code": 404,
            "path": req.path(),
            "method": req.method().as_str()
        }
    }))
}
