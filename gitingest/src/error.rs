use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitingestError {
    #[error("Repository not found: {0}")]
    RepositoryNotFound(String),
    
    #[error("Invalid repository URL: {0}")]
    InvalidRepositoryUrl(String),
    
    #[error("Git operation failed: {0}")]
    GitOperationFailed(String),
    
    #[error("File system error: {0}")]
    FileSystemError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("HTTP client error: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("URL parsing error: {0}")]
    UrlParseError(#[from] url::ParseError),
    
    #[error("Pattern matching error: {0}")]
    PatternError(String),
    
    #[error("Token validation error: {0}")]
    TokenValidationError(String),
    
    #[error("Timeout error: operation timed out after {0} seconds")]
    TimeoutError(u64),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Internal server error: {0}")]
    InternalError(String),
}

impl From<git2::Error> for GitingestError {
    fn from(err: git2::Error) -> Self {
        GitingestError::GitOperationFailed(err.to_string())
    }
}

impl From<anyhow::Error> for GitingestError {
    fn from(err: anyhow::Error) -> Self {
        GitingestError::InternalError(err.to_string())
    }
}

// Web-specific error handling is now implemented in gitingest-server

impl Serialize for GitingestError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("GitingestError", 2)?;
        state.serialize_field("error", &self.to_string())?;
        state.serialize_field("type", &format!("{:?}", self))?;
        state.end()
    }
}

pub type Result<T> = std::result::Result<T, GitingestError>;