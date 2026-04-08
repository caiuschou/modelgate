use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::{errors::ApiError, session_auth, AppState};

#[derive(Deserialize)]
pub struct ChangePasswordBody {
    pub new_password: String,
}

pub async fn change_my_password(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<ChangePasswordBody>,
) -> Result<HttpResponse, ApiError> {
    let user_id = session_auth::resolve_console_session(&req, &state)?.user_id;
    state
        .user_service
        .change_my_password(user_id, body.new_password.trim())?;
    Ok(HttpResponse::NoContent().finish())
}
