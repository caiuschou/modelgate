#[derive(Debug)]
pub enum RepositoryError {
    PoolUnavailable,
    NotFound(String),
    Conflict(String),
    Forbidden(String),
    Internal(String),
}

#[derive(Debug)]
pub enum ServiceError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    TooManyRequests(String),
    Internal(String),
}

impl From<RepositoryError> for ServiceError {
    fn from(value: RepositoryError) -> Self {
        match value {
            RepositoryError::PoolUnavailable => {
                ServiceError::Internal("Database unavailable".into())
            }
            RepositoryError::NotFound(msg) => ServiceError::NotFound(msg),
            RepositoryError::Conflict(msg) => ServiceError::Conflict(msg),
            RepositoryError::Forbidden(msg) => ServiceError::Forbidden(msg),
            RepositoryError::Internal(msg) => ServiceError::Internal(msg),
        }
    }
}
