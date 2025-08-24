use crate::error::{GitingestError, Result};
use crate::models::PatternMatcher;
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::path::Path;

pub struct PatternService;

impl PatternService {
    pub fn new_matcher(
        include_patterns: Vec<String>,
        exclude_patterns: Vec<String>,
    ) -> Result<PatternMatcher> {
        Ok(PatternMatcher {
            include_patterns,
            exclude_patterns,
            gitignore_patterns: Vec::new(),
        })
    }

    pub fn should_include_file<P: AsRef<Path>>(
        matcher: &PatternMatcher,
        file_path: P,
    ) -> Result<bool> {
        let path_ref = file_path.as_ref();

        // If we have include patterns, the file must match at least one
        if !matcher.include_patterns.is_empty() {
            let include_set = Self::build_glob_set(&matcher.include_patterns)?;
            if !include_set.is_match(path_ref) {
                return Ok(false);
            }
        }

        // Check exclude patterns
        if !matcher.exclude_patterns.is_empty() {
            let exclude_set = Self::build_glob_set(&matcher.exclude_patterns)?;
            if exclude_set.is_match(path_ref) {
                return Ok(false);
            }
        }

        // Check gitignore patterns
        if !matcher.gitignore_patterns.is_empty() {
            let gitignore_set = Self::build_glob_set(&matcher.gitignore_patterns)?;
            if gitignore_set.is_match(path_ref) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub fn should_include_directory<P: AsRef<Path>>(
        matcher: &PatternMatcher,
        dir_path: P,
    ) -> Result<bool> {
        let path_ref = dir_path.as_ref();
        let path_str = path_ref.to_string_lossy();

        // Always include directories for traversal, unless explicitly excluded
        if !matcher.exclude_patterns.is_empty() {
            let exclude_set = Self::build_glob_set(&matcher.exclude_patterns)?;
            if exclude_set.is_match(path_ref) || exclude_set.is_match(&format!("{}/", path_str)) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn build_glob_set(patterns: &[String]) -> Result<GlobSet> {
        let mut builder = GlobSetBuilder::new();
        
        for pattern in patterns {
            let glob = Glob::new(pattern)
                .map_err(|e| GitingestError::PatternError(format!("Invalid glob pattern '{}': {}", pattern, e)))?;
            builder.add(glob);
        }

        builder.build()
            .map_err(|e| GitingestError::PatternError(format!("Failed to build glob set: {}", e)))
    }

    pub fn parse_gitignore<P: AsRef<Path>>(gitignore_path: P) -> Result<Vec<String>> {
        let content = std::fs::read_to_string(gitignore_path)?;
        let mut patterns = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Handle negation patterns
            let pattern = if line.starts_with('!') {
                // For now, we'll treat negation patterns as include patterns
                // This is a simplified implementation
                continue;
            } else {
                line.to_string()
            };

            patterns.push(pattern);
        }

        Ok(patterns)
    }

    pub fn add_gitignore_patterns(
        matcher: &mut PatternMatcher,
        gitignore_path: &Path,
    ) -> Result<()> {
        if gitignore_path.exists() {
            let patterns = Self::parse_gitignore(gitignore_path)?;
            matcher.gitignore_patterns.extend(patterns);
        }
        Ok(())
    }
}

pub fn normalize_pattern(pattern: &str) -> String {
    let mut normalized = pattern.to_string();
    
    // Convert Windows-style paths to Unix-style
    normalized = normalized.replace('\\', "/");
    
    // Handle directory patterns
    if normalized.ends_with('/') {
        normalized = format!("{}**", normalized);
    }
    
    normalized
}

pub fn is_binary_file<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    
    // Check file extension
    if let Some(extension) = path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        matches!(ext.as_str(), 
            "exe" | "dll" | "so" | "dylib" | "a" | "lib" | "o" | "obj" |
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "svg" |
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" |
            "zip" | "tar" | "gz" | "bz2" | "7z" | "rar" |
            "mp3" | "mp4" | "avi" | "mov" | "wmv" | "flv"
        )
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        let matcher = PatternMatcher {
            include_patterns: vec!["*.rs".to_string()],
            exclude_patterns: vec!["target/**".to_string()],
            gitignore_patterns: vec![],
        };

        assert!(PatternService::should_include_file(&matcher, "src/main.rs").unwrap());
        assert!(!PatternService::should_include_file(&matcher, "target/debug/main").unwrap());
        assert!(!PatternService::should_include_file(&matcher, "README.md").unwrap());
    }

    #[test]
    fn test_binary_file_detection() {
        assert!(is_binary_file("test.exe"));
        assert!(is_binary_file("image.png"));
        assert!(!is_binary_file("source.rs"));
        assert!(!is_binary_file("README.md"));
    }
}