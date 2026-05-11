use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum AppError {
    NotFound(String),
    ValidationError(String),
    DatabaseError(String),
    IoError(String),
}

pub type AppResult<T> = Result<T, AppError>;

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for AppError {}

impl From<libsql::Error> for AppError {
    fn from(e: libsql::Error) -> Self {
        AppError::DatabaseError(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::IoError(e.to_string())
    }
}
