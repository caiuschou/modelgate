use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde_json::json;
use std::fmt;

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    Conflict(String),
    NotFound(String),
    InternalError(String),
}

impl ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_type(&self) -> &'static str {
        match self {
            ApiError::BadRequest(_) => "validation_error",
            ApiError::Unauthorized(_) => "authentication_error",
            ApiError::Conflict(_) => "conflict_error",
            ApiError::NotFound(_) => "not_found_error",
            ApiError::InternalError(_) => "internal_error",
        }
    }

    fn message(&self) -> &str {
        match self {
            ApiError::BadRequest(message)
            | ApiError::Unauthorized(message)
            | ApiError::Conflict(message)
            | ApiError::NotFound(message)
            | ApiError::InternalError(message) => message,
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error_type(), self.message())
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.status_code()
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).json(json!({
            "error": {
                "message": self.message(),
                "type": self.error_type(),
            }
        }))
    }
}

impl From<crate::services::error::ServiceError> for ApiError {
    fn from(value: crate::services::error::ServiceError) -> Self {
        match value {
            crate::services::error::ServiceError::BadRequest(msg) => ApiError::BadRequest(msg),
            crate::services::error::ServiceError::Unauthorized(msg) => ApiError::Unauthorized(msg),
            crate::services::error::ServiceError::NotFound(msg) => ApiError::NotFound(msg),
            crate::services::error::ServiceError::Conflict(msg) => ApiError::Conflict(msg),
            crate::services::error::ServiceError::Internal(msg) => ApiError::InternalError(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{body::to_bytes, http::StatusCode};
    use serde_json::Value;

    #[actix_web::test]
    async fn bad_request_error_response() {
        let err = ApiError::BadRequest("oops".into());
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
        let resp = err.error_response();
        let bytes = to_bytes(resp.into_body()).await.expect("body bytes");
        let body: Value = serde_json::from_slice(&bytes).expect("parse body");
        assert_eq!(body["error"]["message"], "oops");
        assert_eq!(body["error"]["type"], "validation_error");
    }

    #[actix_web::test]
    async fn all_error_types_are_mapped() {
        let cases = vec![
            (
                ApiError::Unauthorized("u".into()),
                "authentication_error",
                StatusCode::UNAUTHORIZED,
            ),
            (
                ApiError::Conflict("c".into()),
                "conflict_error",
                StatusCode::CONFLICT,
            ),
            (
                ApiError::NotFound("n".into()),
                "not_found_error",
                StatusCode::NOT_FOUND,
            ),
            (
                ApiError::InternalError("i".into()),
                "internal_error",
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];

        for (err, err_type, status) in cases {
            assert_eq!(err.status_code(), status);
            let resp = err.error_response();
            let bytes = to_bytes(resp.into_body()).await.expect("body bytes");
            let body: Value = serde_json::from_slice(&bytes).expect("parse body");
            assert_eq!(body["error"]["type"], err_type);
        }
    }

    #[test]
    fn display_includes_type_and_message() {
        let err = ApiError::BadRequest("oops".into());
        assert_eq!(format!("{}", err), "validation_error: oops");
    }
}
