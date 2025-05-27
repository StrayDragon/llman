use thiserror::Error;
use anyhow::Result as _Result;

#[derive(Error, Debug)]
pub enum LlmanError {
    #[error("Config Error: {message}")]
    Config { message: String },

    #[error("Invalid App Type: {app}")]
    InvalidApp { app: String },

    #[error("Current directory is not a valid project directory")]
    NotProjectDirectory,

    #[error("Cannot generate rules in home directory")]
    HomeDirectoryNotAllowed,

    #[error("Rule file not found: {name}")]
    RuleNotFound { name: String },

    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Inquire Error: {0}")]
    Inquire(#[from] inquire::InquireError),
}

impl LlmanError {
    pub fn display_localized(&self) -> String {
        match self {
            LlmanError::Config { message } => {
                t!("errors.config_error", message = message).to_string()
            }
            LlmanError::InvalidApp { app } => t!("errors.invalid_app", app = app).to_string(),
            LlmanError::NotProjectDirectory => t!("errors.not_project_directory").to_string(),
            LlmanError::HomeDirectoryNotAllowed => {
                t!("errors.home_directory_not_allowed").to_string()
            }
            LlmanError::RuleNotFound { name } => {
                t!("errors.rule_not_found", name = name).to_string()
            }
            _ => self.to_string(),
        }
    }
}

pub type LlmanResult<T> = _Result<T, LlmanError>;