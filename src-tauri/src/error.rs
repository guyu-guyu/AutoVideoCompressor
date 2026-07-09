use serde::Serialize;

/// Application-wide error type. Serializable so Tauri commands can return it
/// straight to the frontend (invoke() rejects with this shape).
#[derive(Debug, Serialize)]
pub struct AppError {
    pub message: String,
}

impl AppError {
    pub fn new(msg: impl Into<String>) -> Self {
        AppError { message: msg.into() }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self { AppError::new(e.to_string()) }
}
impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self { AppError::new(e.to_string()) }
}

pub type AppResult<T> = Result<T, AppError>;
