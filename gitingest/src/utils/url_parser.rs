use crate::error::{GitingestError, Result};
use crate::models::Repository;
use regex::Regex;
use url::Url;

pub struct UrlParser;

impl UrlParser {
    pub fn parse_git_url(input: &str) -> Result<Repository> {
        let trimmed_input = input.trim();
        
        // Try to parse as a direct URL first
        if let Ok(url) = Url::parse(trimmed_input) {
            return Self::parse_url(url);
        }
        
        // Handle GitHub shorthand (owner/repo)
        if let Some(caps) = Regex::new(r"^([a-zA-Z0-9_.-]+)/([a-zA-Z0-9_.-]+)$")
            .unwrap()
            .captures(trimmed_input)
        {
            let owner = caps[1].to_string();
            let repo = caps[2].to_string();
            return Ok(Repository {
                url: format!("https://github.com/{}/{}", owner, repo),
                host: "github.com".to_string(),
                owner,
                name: repo,
                branch: None,
                commit: None,
                subpath: String::new(),
            });
        }
        
        // Try to construct a GitHub URL if it looks like a repository reference
        if !trimmed_input.contains("://") {
            let github_url = format!("https://github.com/{}", trimmed_input);
            if let Ok(url) = Url::parse(&github_url) {
                return Self::parse_url(url);
            }
        }
        
        Err(GitingestError::InvalidRepositoryUrl(
            format!("Unable to parse repository URL: {}", trimmed_input)
        ))
    }
    
    fn parse_url(url: Url) -> Result<Repository> {
        let host = url.host_str()
            .ok_or_else(|| GitingestError::InvalidRepositoryUrl("No host found".to_string()))?
            .to_string();
        
        let path_segments: Vec<&str> = url.path_segments()
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
        let (branch, subpath) = if path_segments.len() > 3 && 
            (path_segments[2] == "tree" || path_segments[2] == "blob") 
        {
            let branch = if path_segments.len() > 3 {
                Some(path_segments[3].to_string())
            } else {
                None
            };
            let subpath = if path_segments.len() > 4 {
                path_segments[4..].join("/")
            } else {
                String::new()
            };
            (branch, subpath)
        } else {
            (None, String::new())
        };
        
        // Construct clean repository URL without tree/blob paths
        let clean_url = format!("https://{}/{}/{}", host, owner, repo_name);
        
        Ok(Repository {
            url: clean_url,
            host,
            owner,
            name: repo_name,
            branch,
            commit: None,
            subpath,
        })
    }
    
    pub fn is_valid_github_url(url: &str) -> bool {
        if let Ok(parsed_url) = Url::parse(url) {
            if let Some(host) = parsed_url.host_str() {
                return host == "github.com" || host == "www.github.com";
            }
        }
        false
    }
    
    pub fn is_valid_git_url(url: &str) -> bool {
        if let Ok(parsed_url) = Url::parse(url) {
            return parsed_url.scheme() == "https" || parsed_url.scheme() == "http" || 
                   parsed_url.scheme() == "git" || parsed_url.scheme() == "ssh";
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_shorthand() {
        let result = UrlParser::parse_git_url("owner/repo").unwrap();
        assert_eq!(result.owner, "owner");
        assert_eq!(result.name, "repo");
        assert_eq!(result.host, "github.com");
        assert_eq!(result.url, "https://github.com/owner/repo");
    }

    #[test]
    fn test_full_github_url() {
        let result = UrlParser::parse_git_url("https://github.com/owner/repo").unwrap();
        assert_eq!(result.owner, "owner");
        assert_eq!(result.name, "repo");
        assert_eq!(result.host, "github.com");
    }

    #[test]
    fn test_github_url_with_branch() {
        let result = UrlParser::parse_git_url("https://github.com/owner/repo/tree/main").unwrap();
        assert_eq!(result.owner, "owner");
        assert_eq!(result.name, "repo");
        assert_eq!(result.branch, Some("main".to_string()));
    }
}