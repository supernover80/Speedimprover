//! Errors from parsing SpeedImprover's data files.

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid TOML: {0}")]
    Toml(#[from] toml::de::Error),
}
