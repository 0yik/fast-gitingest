use crate::config::AppConfig;
use crate::error::{GitingestError, Result};
use crate::models::{CloneConfig, IngestRequest, IngestResponse, IngestStatus, PatternMatcher, ProcessingResult, ProcessingStats};
use crate::utils::{FileService, GitService, PatternService, UrlParser, format_file_size};
use std::time::Instant;
use tempfile::TempDir;
use uuid::Uuid;

pub struct IngestService;

impl IngestService {
    pub async fn process_repository(
        request: IngestRequest,
        config: &AppConfig,
    ) -> Result<IngestResponse> {
        let start_time = Instant::now();
        let id = Uuid::new_v4();
        
        // Parse the repository URL
        let repository = UrlParser::parse_git_url(&request.input_text)?;
        
        // Create temporary directory for cloning
        let temp_dir = TempDir::new()
            .map_err(|e| GitingestError::FileSystemError(format!("Failed to create temp dir: {}", e)))?;
        
        let local_path = temp_dir.path().join(&repository.name);
        
        // Create clone configuration
        let clone_config = CloneConfig {
            url: repository.url.clone(),
            local_path: local_path.clone(),
            branch: request.branch.or(repository.branch.clone()),
            commit: repository.commit.clone(),
            subpath: repository.subpath.clone(),
            include_submodules: request.include_submodules.unwrap_or(false),
            token: request.token,
        };
        
        // Clone the repository
        let clone_start = Instant::now();
        GitService::clone_repository(&clone_config).await?;
        let clone_duration = clone_start.elapsed();
        log::info!("Repository cloning phase completed in {:.2}s", clone_duration.as_secs_f64());
        
        // Create pattern matcher
        let mut matcher = PatternMatcher::default();
        
        // Add user-specified patterns
        if let Some(pattern) = request.pattern {
            match request.pattern_type {
                Some(crate::models::PatternType::Include) => {
                    matcher.include_patterns.push(pattern);
                }
                Some(crate::models::PatternType::Exclude) | None => {
                    matcher.exclude_patterns.push(pattern);
                }
            }
        }
        
        // Add gitignore patterns
        let gitignore_path = local_path.join(".gitignore");
        PatternService::add_gitignore_patterns(&mut matcher, &gitignore_path)?;
        
        // Set limits from config and request
        let max_file_size = request.max_file_size.unwrap_or(config.max_file_size);
        
        // Scan the repository with memory-efficient lazy loading
        log::info!("Starting memory-efficient lazy file scanning...");
        let scan_start = Instant::now();
        let file_tree_lazy = FileService::scan_directory_lazy(
            &local_path,
            &matcher,
            max_file_size,
            config.max_files,
            config.max_directory_depth,
            config.concurrent_file_limit,
            config.batch_size,
        ).await?;
        let scan_duration = scan_start.elapsed();
        log::info!("Lazy file scanning completed in {:.2}s", scan_duration.as_secs_f64());
        
        // Generate tree string (lightweight)
        log::info!("Starting tree generation...");
        let generation_start = Instant::now();
        let tree = FileService::generate_tree_string_lazy(&file_tree_lazy, "", true);
        let generation_duration = generation_start.elapsed();
        log::info!("Tree generation completed in {:.2}s", generation_duration.as_secs_f64());
        
        // Calculate statistics from lazy tree
        let files_analyzed = Self::count_files_lazy(&file_tree_lazy);
        let total_size_bytes = Self::calculate_total_size_lazy(&file_tree_lazy);
        let processing_time = start_time.elapsed();
        
        // Write content to file directly (streaming approach)
        log::info!("Starting streaming content write...");
        let content_start = Instant::now();
        let temp_content_path = local_path.join("temp_content.txt");
        FileService::write_content_to_file(&file_tree_lazy, &temp_content_path)?;
        
        // Read back only for response (could be optimized further by not reading back)
        let content = std::fs::read_to_string(&temp_content_path)
            .unwrap_or_else(|_| "Error reading generated content".to_string());
        let content_duration = content_start.elapsed();
        log::info!("Streaming content write completed in {:.2}s", content_duration.as_secs_f64());
        
        // Clean up temp file
        let _ = std::fs::remove_file(&temp_content_path);
        
        // Estimate tokens
        let estimated_tokens = Self::estimate_tokens(&content);
        
        // Create processing result
        let processing_result = ProcessingResult {
            summary: Self::generate_summary(&repository, files_analyzed, total_size_bytes),
            tree: tree.clone(),
            content: content.clone(),
            stats: ProcessingStats {
                files_analyzed,
                total_size_bytes,
                estimated_tokens,
                processing_time_ms: processing_time.as_millis() as u64,
            },
        };
        
        // Create response
        let response = IngestResponse {
            id,
            repo_url: repository.url.clone(),
            short_repo_url: Self::create_short_url(&repository),
            summary: processing_result.summary,
            digest_url: None, // Would be implemented for actual digest storage
            tree,
            content,
            status: IngestStatus::Completed,
        };
        
        let total_processing_time = start_time.elapsed();
        log::info!(
            "Repository ingestion completed successfully - Total time: {:.2}s (Clone: {:.2}s, Scan: {:.2}s, Tree: {:.2}s, Content: {:.2}s)", 
            total_processing_time.as_secs_f64(),
            clone_duration.as_secs_f64(),
            scan_duration.as_secs_f64(),
            generation_duration.as_secs_f64(),
            content_duration.as_secs_f64()
        );
        
        Ok(response)
    }
    
    fn generate_summary(repository: &crate::models::Repository, files_count: usize, total_size: u64) -> String {
        format!(
            "Repository: {}/{}\nFiles processed: {}\nTotal size: {}\nHost: {}",
            repository.owner,
            repository.name,
            files_count,
            format_file_size(total_size),
            repository.host
        )
    }
    
    fn create_short_url(repository: &crate::models::Repository) -> String {
        format!("{}/{}", repository.owner, repository.name)
    }
    
    fn estimate_tokens(content: &str) -> Option<usize> {
        // Rough estimation: ~4 characters per token for English text
        Some(content.len() / 4)
    }
    
    fn count_files_lazy(node: &crate::models::FileNodeLazy) -> usize {
        match node.node_type {
            crate::models::FileNodeType::File => 1,
            crate::models::FileNodeType::Directory => {
                node.children.iter().map(|child| Self::count_files_lazy(child)).sum()
            }
            crate::models::FileNodeType::Symlink => 0,
        }
    }

    fn calculate_total_size_lazy(node: &crate::models::FileNodeLazy) -> u64 {
        match node.node_type {
            crate::models::FileNodeType::File => node.size,
            crate::models::FileNodeType::Directory => {
                node.children.iter().map(|child| Self::calculate_total_size_lazy(child)).sum()
            }
            crate::models::FileNodeType::Symlink => 0,
        }
    }
}