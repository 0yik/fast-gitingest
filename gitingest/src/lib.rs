pub mod config;
pub mod error;
pub mod models;
pub mod services;
pub mod utils;

pub use config::AppConfig;
pub use error::GitingestError;
pub use models::*;
pub use services::*;
pub use utils::*;

pub type Result<T> = std::result::Result<T, GitingestError>;