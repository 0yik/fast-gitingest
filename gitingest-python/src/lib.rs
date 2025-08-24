use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use pyo3::types::PyType;
use pyo3_asyncio::tokio::future_into_py;
use gitingest::{AppConfig, IngestService, IngestRequest, DownloadFormat};
use serde_json;
use std::collections::HashMap;
use uuid::Uuid;

/// Python class for configuring the gitingest processing
#[pyclass]
#[derive(Clone)]
pub struct GitingestConfig {
    inner: AppConfig,
}

#[pymethods]
impl GitingestConfig {
    #[new]
    #[pyo3(signature = (max_file_size=None, max_files=None, max_total_size=None, max_directory_depth=None, default_timeout=None, allowed_hosts=None))]
    pub fn new(
        max_file_size: Option<u64>,
        max_files: Option<usize>,
        max_total_size: Option<u64>,
        max_directory_depth: Option<usize>,
        default_timeout: Option<u64>,
        allowed_hosts: Option<Vec<String>>,
    ) -> Self {
        let mut config = AppConfig::from_env().unwrap_or_default();
        
        if let Some(size) = max_file_size {
            config.max_file_size = size;
        }
        if let Some(files) = max_files {
            config.max_files = files;
        }
        if let Some(size) = max_total_size {
            config.max_total_size = size;
        }
        if let Some(depth) = max_directory_depth {
            config.max_directory_depth = depth as u32;
        }
        if let Some(timeout) = default_timeout {
            config.default_timeout = timeout;
        }
        if let Some(hosts) = allowed_hosts {
            config.allowed_hosts = hosts;
        }
        
        Self { inner: config }
    }
    
    /// Create config from environment variables
    #[classmethod]
    pub fn from_env(_cls: &PyType) -> PyResult<Self> {
        let config = AppConfig::from_env()
            .map_err(|e| PyValueError::new_err(format!("Failed to load config: {}", e)))?;
        Ok(Self { inner: config })
    }
}

/// Main gitingest class for processing repositories
#[pyclass]
pub struct Gitingest {
    config: AppConfig,
}

#[pymethods]
impl Gitingest {
    #[new]
    #[pyo3(signature = (config=None))]
    pub fn new(config: Option<GitingestConfig>) -> Self {
        let config = match config {
            Some(c) => c.inner,
            None => AppConfig::from_env().unwrap_or_default(),
        };
        
        Self { config }
    }
    
    /// Ingest a repository and return the result
    #[pyo3(signature = (
        input,
        format="text",
        include_patterns=None,
        exclude_patterns=None,
        max_file_size=None,
        max_files=None,
        token=None,
        branch=None,
        include_submodules=None
    ))]
    pub fn ingest<'py>(
        &self,
        py: Python<'py>,
        input: String,
        format: &str,
        include_patterns: Option<Vec<String>>,
        exclude_patterns: Option<Vec<String>>,
        max_file_size: Option<u64>,
        max_files: Option<usize>,
        token: Option<String>,
        branch: Option<String>,
        include_submodules: Option<bool>,
    ) -> PyResult<&'py PyAny> {
        let config = self.config.clone();
        
        let download_format = match format.to_lowercase().as_str() {
            "json" => DownloadFormat::Json,
            "markdown" | "md" => DownloadFormat::Markdown,
            "text" | "txt" => DownloadFormat::Text,
            _ => return Err(PyValueError::new_err("Invalid format. Use 'json', 'text', or 'markdown'")),
        };
        
        let request = IngestRequest {
            input_text: input,
            download_format: Some(download_format),
            include_patterns,
            exclude_patterns,
            max_file_size,
            max_files,
            pattern_type: None,
            pattern: None,
            token,
            branch,
            include_submodules,
        };
        
        future_into_py(py, async move {
            let id = Uuid::new_v4();
            let response = IngestService::process_repository(request, &config, id)
                .await
                .map_err(|e| PyValueError::new_err(format!("Ingestion failed: {}", e)))?;
            
            // Convert to Python dictionary
            let mut result = HashMap::new();
            result.insert("short_repo_url", response.short_repo_url);
            result.insert("summary", response.summary);
            result.insert("tree", response.tree);
            result.insert("content", response.content);
            
            Ok(result)
        })
    }
    
    /// Ingest a repository and return JSON string
    #[pyo3(signature = (
        input,
        include_patterns=None,
        exclude_patterns=None,
        max_file_size=None,
        max_files=None,
        token=None,
        branch=None,
        include_submodules=None
    ))]
    pub fn ingest_json<'py>(
        &self,
        py: Python<'py>,
        input: String,
        include_patterns: Option<Vec<String>>,
        exclude_patterns: Option<Vec<String>>,
        max_file_size: Option<u64>,
        max_files: Option<usize>,
        token: Option<String>,
        branch: Option<String>,
        include_submodules: Option<bool>,
    ) -> PyResult<&'py PyAny> {
        let config = self.config.clone();
        
        let request = IngestRequest {
            input_text: input,
            download_format: Some(DownloadFormat::Json),
            include_patterns,
            exclude_patterns,
            max_file_size,
            max_files,
            pattern_type: None,
            pattern: None,
            token,
            branch,
            include_submodules,
        };
        
        future_into_py(py, async move {
            let id = Uuid::new_v4();
            let response = IngestService::process_repository(request, &config, id)
                .await
                .map_err(|e| PyValueError::new_err(format!("Ingestion failed: {}", e)))?;
            
            let json_str = serde_json::to_string_pretty(&response)
                .map_err(|e| PyValueError::new_err(format!("JSON serialization failed: {}", e)))?;
            
            Ok(json_str)
        })
    }
}

/// CLI function that mimics the main CLI interface
#[pyfunction]
#[pyo3(signature = (args))]
pub fn cli(args: Vec<String>) -> PyResult<String> {
    // This is a simplified version - you might want to implement full CLI parsing
    if args.is_empty() {
        return Ok("Usage: gitingest.cli(['ingest', 'repo_url'])".to_string());
    }
    
    match args[0].as_str() {
        "platforms" => Ok("Supported platforms: GitHub, GitLab, Bitbucket".to_string()),
        "config" => Ok("Configuration loaded from environment variables".to_string()),
        _ => Ok("Use ingest() method for repository processing".to_string()),
    }
}

/// Python module definition
#[pymodule]
fn gitingest_python(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Gitingest>()?;
    m.add_class::<GitingestConfig>()?;
    m.add_function(wrap_pyfunction!(cli, m)?)?;
    Ok(())
}