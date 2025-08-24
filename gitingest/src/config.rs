use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub max_file_size: u64,
    pub max_files: usize,
    pub max_total_size: u64,
    pub max_directory_depth: u32,
    pub default_timeout: u64,
    pub temp_dir: String,
    pub github_token: Option<String>,
    pub allowed_hosts: Vec<String>,
    pub concurrent_file_limit: usize,
    pub batch_size: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            max_file_size: u64::MAX,
            max_files: usize::MAX,
            max_total_size: u64::MAX,
            max_directory_depth: u32::MAX,
            default_timeout: 60,
            temp_dir: "/tmp/fast-gitingest".to_string(),
            github_token: None,
            allowed_hosts: vec![
                "github.com".to_string(),
                "gitlab.com".to_string(),
                "bitbucket.org".to_string(),
            ],
            concurrent_file_limit: 1000,
            batch_size: 500,
        }
    }
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let mut config = Self::default();

        if let Ok(host) = env::var("HOST") {
            config.host = host;
        }

        if let Ok(port) = env::var("PORT") {
            config.port = port.parse()?;
        }

        if let Ok(max_file_size) = env::var("MAX_FILE_SIZE") {
            config.max_file_size = max_file_size.parse()?;
        }

        if let Ok(max_files) = env::var("MAX_FILES") {
            config.max_files = max_files.parse()?;
        }

        if let Ok(max_total_size) = env::var("MAX_TOTAL_SIZE") {
            config.max_total_size = max_total_size.parse()?;
        }

        if let Ok(max_directory_depth) = env::var("MAX_DIRECTORY_DEPTH") {
            config.max_directory_depth = max_directory_depth.parse()?;
        }

        if let Ok(default_timeout) = env::var("DEFAULT_TIMEOUT") {
            config.default_timeout = default_timeout.parse()?;
        }

        if let Ok(temp_dir) = env::var("TEMP_DIR") {
            config.temp_dir = temp_dir;
        }

        config.github_token = env::var("GITHUB_TOKEN").ok();

        if let Ok(allowed_hosts) = env::var("ALLOWED_HOSTS") {
            config.allowed_hosts = allowed_hosts
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }

        if let Ok(concurrent_file_limit) = env::var("CONCURRENT_FILE_LIMIT") {
            config.concurrent_file_limit = concurrent_file_limit.parse()?;
        }

        if let Ok(batch_size) = env::var("BATCH_SIZE") {
            config.batch_size = batch_size.parse()?;
        }

        Ok(config)
    }
}