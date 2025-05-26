use thiserror::Error;

#[derive(Error, Debug)]
pub enum LlmanError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

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

    #[error("Interactive prompt error: {0}")]
    Inquire(#[from] inquire::InquireError),
}

pub type Result<T> = std::result::Result<T, LlmanError>;
