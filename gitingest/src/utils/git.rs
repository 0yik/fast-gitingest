use crate::error::{GitingestError, Result};
use crate::models::{CloneConfig, Repository};
use git2::{Repository as Git2Repository};
use std::path::Path;
use std::time::Instant;
use url::Url;

pub struct GitService;

impl GitService {
    pub async fn clone_repository(config: &CloneConfig) -> Result<()> {
        let start_time = Instant::now();
        log::info!("Starting git clone of {} to {:?}", config.url, config.local_path);
        
        let repo_path = &config.local_path;
        
        // Create parent directories if they don't exist
        if let Some(parent) = repo_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Use direct git command for shallow clone - much faster than git2

        // Clone the repository with shallow clone for speed
        log::info!("Executing shallow git clone command (depth=1)...");
        let clone_start = Instant::now();
        
        // Build git command arguments for shallow clone
        let mut args = vec![
            "clone".to_string(),
            "--depth=1".to_string(), // Shallow clone - only latest commit
            "--single-branch".to_string(), // Only clone the specified branch
            "--quiet".to_string(), // Reduce output noise
        ];

        // Add branch specification if provided
        if let Some(branch) = &config.branch {
            args.push("--branch".to_string());
            args.push(branch.clone());
        }

        // Prepare URL with authentication if token provided
        let clone_url = if let Some(token) = &config.token {
            // For GitHub, use token as username with empty password
            config.url.replace("https://", &format!("https://{}@", token))
        } else {
            config.url.clone()
        };

        args.push(clone_url);
        args.push(repo_path.to_string_lossy().to_string());

        // Execute git command
        let output = tokio::process::Command::new("git")
            .args(&args)
            .output()
            .await
            .map_err(|e| GitingestError::GitOperationFailed(format!("Git command failed: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(GitingestError::GitOperationFailed(
                format!("Shallow clone failed: {}", error_msg)
            ));
        }

        let clone_duration = clone_start.elapsed();
        let total_duration = start_time.elapsed();
        
        log::info!(
            "Git clone completed successfully - Clone time: {:.2}s, Total time: {:.2}s", 
            clone_duration.as_secs_f64(),
            total_duration.as_secs_f64()
        );

        Ok(())
    }

    pub fn parse_repository_url(url: &str) -> Result<Repository> {
        let parsed_url = Url::parse(url)
            .map_err(|_| GitingestError::InvalidRepositoryUrl(url.to_string()))?;

        let host = parsed_url.host_str()
            .ok_or_else(|| GitingestError::InvalidRepositoryUrl("No host found".to_string()))?
            .to_string();

        let path_segments: Vec<&str> = parsed_url.path_segments()
            .ok_or_else(|| GitingestError::InvalidRepositoryUrl("Invalid path".to_string()))?
            .collect();

        if path_segments.len() < 2 {
            return Err(GitingestError::InvalidRepositoryUrl(
                "URL must contain owner and repository name".to_string()
            ));
        }

        let owner = path_segments[0].to_string();
        let repo_name = path_segments[1].trim_end_matches(".git").to_string();

        // Handle GitHub-style URLs with tree/blob/etc.
        let (branch, subpath) = if path_segments.len() > 3 && path_segments[2] == "tree" {
            let branch = Some(path_segments[3].to_string());
            let subpath = if path_segments.len() > 4 {
                path_segments[4..].join("/")
            } else {
                String::new()
            };
            (branch, subpath)
        } else {
            (None, String::new())
        };

        Ok(Repository {
            url: format!("https://{}/{}/{}", host, owner, repo_name),
            host,
            owner,
            name: repo_name,
            branch,
            commit: None,
            subpath,
        })
    }

    pub async fn check_repository_exists(url: &str, token: Option<&str>) -> Result<bool> {
        let client = reqwest::Client::new();
        let mut request = client.head(url);

        if let Some(token) = token {
            request = request.header("Authorization", format!("token {}", token));
        }

        match request.send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    pub fn validate_github_token(token: &str) -> Result<()> {
        // GitHub tokens should start with specific prefixes and have specific lengths
        if token.starts_with("ghp_") && token.len() == 40 {
            return Ok(());
        }
        if token.starts_with("github_pat_") && token.len() == 93 {
            return Ok(());
        }
        
        Err(GitingestError::TokenValidationError(
            "Invalid GitHub token format".to_string()
        ))
    }
}

pub fn is_git_repository<P: AsRef<Path>>(path: P) -> bool {
    Git2Repository::open(path).is_ok()
}

pub async fn get_repository_info<P: AsRef<Path>>(path: P) -> Result<Option<String>> {
    let repo = Git2Repository::open(path)?;
    
    if let Ok(head) = repo.head() {
        if let Some(oid) = head.target() {
            return Ok(Some(oid.to_string()));
        }
    }
    
    Ok(None)
}