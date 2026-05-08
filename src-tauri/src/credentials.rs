use serde::Deserialize;
use std::path::PathBuf;
use crate::error::{AppError, AppResult};

#[derive(Deserialize, Debug, Clone)]
pub struct Credentials {
    pub turso_url: String,
    pub turso_auth_token: String,
}

pub fn credentials_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("finanzas")
        .join("credentials.toml")
}

pub fn load_credentials() -> AppResult<Credentials> {
    let path = credentials_path();
    let content = std::fs::read_to_string(&path)
        .map_err(|e| AppError::InvalidCredentials(e.to_string()))?;
    toml::from_str(&content).map_err(|e| AppError::InvalidCredentials(e.to_string()))
}

pub fn has_credentials() -> bool {
    credentials_path().exists()
}
