use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRequest {
    pub input_text: String,
    pub max_file_size: Option<u64>,
    pub max_files: Option<usize>,
    pub pattern_type: Option<PatternType>,
    pub pattern: Option<String>,
    pub include_patterns: Option<Vec<String>>,
    pub exclude_patterns: Option<Vec<String>>,
    pub token: Option<String>,
    pub branch: Option<String>,
    pub include_submodules: Option<bool>,
    pub download_format: Option<DownloadFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DownloadFormat {
    #[serde(rename = "txt")]
    Text,
    #[serde(rename = "md")]
    Markdown,
    #[serde(rename = "json")]
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    #[serde(rename = "include")]
    Include,
    #[serde(rename = "exclude")]
    Exclude,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResponse {
    pub id: Uuid,
    pub repo_url: String,
    pub short_repo_url: String,
    pub summary: String,
    pub digest_url: Option<String>,
    pub tree: String,
    pub content: String,
    pub status: IngestStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IngestStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "processing")]
    Processing,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub url: String,
    pub host: String,
    pub owner: String,
    pub name: String,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub subpath: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloneConfig {
    pub url: String,
    pub local_path: PathBuf,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub subpath: String,
    pub include_submodules: bool,
    pub token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FileNode {
    pub name: String,
    pub path: PathBuf,
    pub relative_path: String,
    pub node_type: FileNodeType,
    pub size: u64,
    pub has_content: bool, // Uses lazy loading - content loaded on demand
    pub children: Vec<FileNode>,
    pub depth: u32,
}

use std::io::Write;

pub trait ContentWriter {
    fn write_content(&self, writer: &mut dyn Write) -> std::io::Result<()>;
}

impl ContentWriter for FileNode {
    fn write_content(&self, writer: &mut dyn Write) -> std::io::Result<()> {
        if self.node_type == FileNodeType::File && self.has_content {
            writeln!(writer, "{}:", self.relative_path)?;
            writeln!(writer, "{}", "=".repeat(48))?;
            
            if self.size > 100_000 {
                writeln!(writer, "[Large file content truncated - {} bytes]\n", self.size)?;
            } else {
                match std::fs::read_to_string(&self.path) {
                    Ok(content) => {
                        write!(writer, "{}\n\n", content)?;
                    }
                    Err(_) => {
                        writeln!(writer, "[Error reading file content]\n")?;
                    }
                }
            }
        } else if self.node_type == FileNodeType::Directory {
            for child in &self.children {
                child.write_content(writer)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileNodeType {
    Directory,
    File,
    Symlink,
}

#[derive(Debug, Clone)]
pub struct FileSystemStats {
    pub total_files: usize,
    pub total_size: u64,
    pub processed_files: usize,
    pub skipped_files: usize,
}

impl Default for FileSystemStats {
    fn default() -> Self {
        Self {
            total_files: 0,
            total_size: 0,
            processed_files: 0,
            skipped_files: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingResult {
    pub summary: String,
    pub tree: String,
    pub content: String,
    pub stats: ProcessingStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStats {
    pub files_analyzed: usize,
    pub total_size_bytes: u64,
    pub estimated_tokens: Option<usize>,
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone)]
pub struct PatternMatcher {
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub gitignore_patterns: Vec<String>,
}

impl Default for PatternMatcher {
    fn default() -> Self {
        Self {
            include_patterns: Vec::new(),
            exclude_patterns: default_exclude_patterns(),
            gitignore_patterns: Vec::new(),
        }
    }
}

fn default_exclude_patterns() -> Vec<String> {
    vec![
        // Version control
        ".git".to_string(),
        ".svn".to_string(),
        ".hg".to_string(),
        
        // Build artifacts
        "target/".to_string(),
        "build/".to_string(),
        "dist/".to_string(),
        "node_modules/".to_string(),
        "__pycache__/".to_string(),
        "*.pyc".to_string(),
        
        // IDE and editor files
        ".vscode/".to_string(),
        ".idea/".to_string(),
        "*.swp".to_string(),
        "*.swo".to_string(),
        ".DS_Store".to_string(),
        
        // Logs and temporary files
        "*.log".to_string(),
        "*.tmp".to_string(),
        "*.temp".to_string(),
        
        // Binary files
        "*.exe".to_string(),
        "*.dll".to_string(),
        "*.so".to_string(),
        "*.dylib".to_string(),
        "*.a".to_string(),
        "*.lib".to_string(),
        
        // Media files
        "*.png".to_string(),
        "*.jpg".to_string(),
        "*.jpeg".to_string(),
        "*.gif".to_string(),
        "*.pdf".to_string(),
        "*.mp4".to_string(),
        "*.mp3".to_string(),
        "*.wav".to_string(),
    ]
}