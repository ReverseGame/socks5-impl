use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("Invalid HTTP request: {0}")]
    InvalidRequest(String),

    #[error("Invalid HTTP response: {0}")]
    InvalidResponse(String),

    #[error("Invalid URI: {0}")]
    InvalidUri(String),

    #[error("Invalid header: {0}")]
    InvalidHeader(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[cfg(feature = "async")]
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, HttpError>;
