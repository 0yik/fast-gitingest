use crate::error::{GitingestError, Result};
use crate::models::{FileNode, FileNodeType, FileNodeLazy, ContentWriter};
use crate::utils::patterns::{is_binary_file, PatternService};
use crate::models::PatternMatcher;
use encoding_rs::UTF_8;
use futures::future::join_all;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fs as std_fs};
use tokio::fs;
use tokio::sync::Semaphore;
use walkdir::WalkDir;

pub struct FileService;

impl FileService {
    pub async fn scan_directory<P: AsRef<Path>>(
        path: P,
        matcher: &PatternMatcher,
        max_file_size: u64,
        max_files: usize,
        max_depth: u32,
        concurrent_limit: usize,
        batch_size: usize,
    ) -> Result<FileNode> {
        let path = path.as_ref();
        
        // First, collect all paths using WalkDir
        let discovery_start = std::time::Instant::now();
        let all_paths: Vec<PathBuf> = WalkDir::new(path)
            .max_depth(max_depth as usize)
            .into_iter()
            .filter_map(|entry| {
                entry.ok().map(|e| e.path().to_path_buf())
            })
            .take(max_files)
            .collect();
        let discovery_duration = discovery_start.elapsed();
        log::info!("Path discovery completed in {:.3}s - found {} paths", 
                  discovery_duration.as_secs_f64(), all_paths.len());

        // Group paths by directory for hierarchical processing
        let mut file_map: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        let mut all_files = Vec::new();
        
        for path_buf in all_paths {
            if path_buf.is_file() {
                all_files.push(path_buf.clone());
            }
            if let Some(parent) = path_buf.parent() {
                file_map.entry(parent.to_path_buf())
                    .or_insert_with(Vec::new)
                    .push(path_buf);
            }
        }

        // Process all files in batches for better memory management
        log::info!("Starting batched async processing of {} files with {} concurrent limit and {} batch size", 
                  all_files.len(), concurrent_limit, batch_size);
        let processing_start = std::time::Instant::now();
        
        let mut processed_files: Vec<(PathBuf, Result<FileNode>)> = Vec::with_capacity(all_files.len());
        let semaphore = Arc::new(Semaphore::new(concurrent_limit)); // Control concurrency
        
        // Process files in batches to reduce memory usage
        for chunk in all_files.chunks(batch_size) {
            let futures: Vec<_> = chunk
                .iter()
                .map(|file_path| {
                    let semaphore = semaphore.clone();
                    let path_buf = path.to_path_buf();
                    let file_path = file_path.clone();
                    async move {
                        let _permit = semaphore.acquire().await.unwrap();
                        let result = Self::process_file(
                            &file_path,
                            &path_buf,
                            matcher,
                            max_file_size,
                        ).await;
                        (file_path, result)
                    }
                })
                .collect();

            let mut batch_results = join_all(futures).await;
            processed_files.append(&mut batch_results);
            
            // Log progress
            log::debug!("Processed batch, total files processed: {}", processed_files.len());
        }
        
        let processing_duration = processing_start.elapsed();
        log::info!("Async file processing completed in {:.3}s", 
                  processing_duration.as_secs_f64());

        // Convert results into a file map
        let mut file_nodes: HashMap<PathBuf, FileNode> = HashMap::new();
        for (file_path, result) in processed_files {
            if let Ok(node) = result {
                file_nodes.insert(file_path, node);
            }
        }

        // Build the hierarchical structure
        Self::build_directory_tree(path, &file_nodes, &file_map, &HashMap::new())
    }

    async fn process_file<P: AsRef<Path>>(
        file_path: P,
        root_path: P,
        matcher: &PatternMatcher,
        max_file_size: u64,
    ) -> Result<FileNode> {
        let file_path = file_path.as_ref();
        let root_path = root_path.as_ref();
        
        let metadata = fs::metadata(file_path).await?;
        let name = file_path
            .file_name()
            .unwrap_or_else(|| file_path.as_os_str())
            .to_string_lossy()
            .into_owned();

        let relative_path = file_path
            .strip_prefix(root_path)
            .unwrap_or(file_path)
            .to_string_lossy()
            .into_owned();

        // Quick checks first
        if metadata.len() > max_file_size {
            return Ok(FileNode {
                name,
                path: file_path.to_path_buf(),
                relative_path,
                node_type: FileNodeType::File,
                size: metadata.len(),
                content: Some("File too large to process".to_string()),
                children: Vec::new(),
                depth: 0,
            });
        }

        if !PatternService::should_include_file(matcher, file_path)? {
            return Ok(FileNode {
                name,
                path: file_path.to_path_buf(),
                relative_path,
                node_type: FileNodeType::File,
                size: metadata.len(),
                content: Some("File excluded by patterns".to_string()),
                children: Vec::new(),
                depth: 0,
            });
        }

        if is_binary_file(file_path) {
            return Ok(FileNode {
                name,
                path: file_path.to_path_buf(),
                relative_path,
                node_type: FileNodeType::File,
                size: metadata.len(),
                content: Some("Binary file".to_string()),
                children: Vec::new(),
                depth: 0,
            });
        }

        // Read content asynchronously with size limit check
        let content = if metadata.len() > 1024 * 1024 { // 1MB limit for content inclusion
            "Large file - content not included".to_string()
        } else {
            Self::read_file_content_async(file_path).await.unwrap_or_else(|_| {
                "Error reading file content".to_string()
            })
        };

        Ok(FileNode {
            name,
            path: file_path.to_path_buf(),
            relative_path,
            node_type: FileNodeType::File,
            size: metadata.len(),
            content: Some(content),
            children: Vec::new(),
            depth: 0,
        })
    }


    fn build_directory_tree<P: AsRef<Path>>(
        current_path: P,
        file_nodes: &HashMap<PathBuf, FileNode>,
        file_map: &HashMap<PathBuf, Vec<PathBuf>>,
        built_dirs: &HashMap<PathBuf, FileNode>,
    ) -> Result<FileNode> {
        let current_path = current_path.as_ref();
        let name = current_path
            .file_name()
            .unwrap_or_else(|| current_path.as_os_str())
            .to_string_lossy()
            .into_owned();

        let relative_path = String::new(); // Will be set properly when used

        // Get immediate children of this directory
        let mut children = Vec::new();
        let mut subdirectories = std::collections::HashSet::new();
        
        if let Some(child_paths) = file_map.get(current_path) {
            for child_path in child_paths {
                if child_path.is_file() {
                    // Add file nodes
                    if let Some(child_node) = file_nodes.get(child_path) {
                        children.push(child_node.clone());
                    }
                } else if child_path.is_dir() {
                    // Track subdirectories to process
                    subdirectories.insert(child_path.clone());
                }
            }
        }

        // Find all subdirectories by checking file_map keys
        for (dir_path, _) in file_map {
            if let Some(parent) = dir_path.parent() {
                if parent == current_path && dir_path.is_dir() {
                    subdirectories.insert(dir_path.clone());
                }
            }
        }

        // Recursively build subdirectory nodes
        for subdir_path in subdirectories {
            let subdir_node = Self::build_directory_tree(
                &subdir_path,
                file_nodes,
                file_map,
                built_dirs,
            )?;
            children.push(subdir_node);
        }

        // Sort children: directories first, then files, all alphabetically
        children.sort_by(|a, b| {
            match (a.node_type, b.node_type) {
                (FileNodeType::Directory, FileNodeType::File) => std::cmp::Ordering::Less,
                (FileNodeType::File, FileNodeType::Directory) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        Ok(FileNode {
            name,
            path: current_path.to_path_buf(),
            relative_path,
            node_type: FileNodeType::Directory,
            size: 0,
            content: None,
            children,
            depth: 0,
        })
    }


    pub fn read_file_content<P: AsRef<Path>>(path: P) -> Result<String> {
        let path = path.as_ref();
        let bytes = std_fs::read(path)?;
        
        // Detect encoding
        let (cow, _encoding_used, _had_errors) = UTF_8.decode(&bytes);
        
        // If UTF-8 decoding had errors, try to detect the actual encoding
        if _had_errors {
            // Try common encodings
            for encoding in &[encoding_rs::WINDOWS_1252, encoding_rs::ISO_8859_2] {
                let (cow, _encoding_used, had_errors) = encoding.decode(&bytes);
                if !had_errors {
                    return Ok(cow.into_owned());
                }
            }
            
            // If all else fails, replace invalid sequences
            return Ok(String::from_utf8_lossy(&bytes).into_owned());
        }

        Ok(cow.into_owned())
    }

    pub async fn read_file_content_async<P: AsRef<Path>>(path: P) -> Result<String> {
        let path = path.as_ref();
        let bytes = fs::read(path).await?;
        
        // Detect encoding
        let (cow, _encoding_used, _had_errors) = UTF_8.decode(&bytes);
        
        // If UTF-8 decoding had errors, try to detect the actual encoding
        if _had_errors {
            // Try common encodings
            for encoding in &[encoding_rs::WINDOWS_1252, encoding_rs::ISO_8859_2] {
                let (cow, _encoding_used, had_errors) = encoding.decode(&bytes);
                if !had_errors {
                    return Ok(cow.into_owned());
                }
            }
            
            // If all else fails, replace invalid sequences
            return Ok(String::from_utf8_lossy(&bytes).into_owned());
        }

        Ok(cow.into_owned())
    }

    pub fn generate_tree_string(node: &FileNode, prefix: &str, is_last: bool) -> String {
        let mut result = String::new();
        
        let connector = if is_last { "└── " } else { "├── " };
        let name_display = match node.node_type {
            FileNodeType::Directory => format!("{}/", node.name),
            FileNodeType::Symlink => format!("{} -> ?", node.name),
            FileNodeType::File => node.name.clone(),
        };
        
        result.push_str(&format!("{}{}{}\n", prefix, connector, name_display));
        
        if node.node_type == FileNodeType::Directory {
            let new_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
            
            for (i, child) in node.children.iter().enumerate() {
                let is_child_last = i == node.children.len() - 1;
                result.push_str(&Self::generate_tree_string(child, &new_prefix, is_child_last));
            }
        }
        
        result
    }


    pub fn count_files(node: &FileNode) -> usize {
        match node.node_type {
            FileNodeType::File => 1,
            FileNodeType::Directory => {
                node.children.iter().map(|child| Self::count_files(child)).sum()
            }
            FileNodeType::Symlink => 0,
        }
    }

    pub fn calculate_total_size(node: &FileNode) -> u64 {
        match node.node_type {
            FileNodeType::File => node.size,
            FileNodeType::Directory => {
                node.children.iter().map(|child| Self::calculate_total_size(child)).sum()
            }
            FileNodeType::Symlink => 0,
        }
    }

    pub async fn scan_directory_lazy<P: AsRef<Path>>(
        path: P,
        matcher: &PatternMatcher,
        max_file_size: u64,
        max_files: usize,
        max_depth: u32,
        concurrent_limit: usize,
        batch_size: usize,
    ) -> Result<FileNodeLazy> {
        let path = path.as_ref();
        
        let discovery_start = std::time::Instant::now();
        let all_paths: Vec<PathBuf> = WalkDir::new(path)
            .max_depth(max_depth as usize)
            .into_iter()
            .filter_map(|entry| {
                entry.ok().map(|e| e.path().to_path_buf())
            })
            .take(max_files)
            .collect();
        let discovery_duration = discovery_start.elapsed();
        log::info!("Lazy path discovery completed in {:.3}s - found {} paths", 
                  discovery_duration.as_secs_f64(), all_paths.len());

        let mut file_map: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        let mut all_files = Vec::new();
        
        for path_buf in all_paths {
            if path_buf.is_file() {
                all_files.push(path_buf.clone());
            }
            if let Some(parent) = path_buf.parent() {
                file_map.entry(parent.to_path_buf())
                    .or_insert_with(Vec::new)
                    .push(path_buf);
            }
        }

        // Only process metadata, no content loading
        log::info!("Starting lazy metadata processing of {} files", all_files.len());
        let processing_start = std::time::Instant::now();
        
        let mut file_nodes: HashMap<PathBuf, FileNodeLazy> = HashMap::new();
        let semaphore = Arc::new(Semaphore::new(concurrent_limit));
        
        for chunk in all_files.chunks(batch_size) {
            let futures: Vec<_> = chunk
                .iter()
                .map(|file_path| {
                    let semaphore = semaphore.clone();
                    let file_path = file_path.clone();
                    let path_buf = path.to_path_buf();
                    async move {
                        let _permit = semaphore.acquire().await.unwrap();
                        let result = Self::process_file_lazy(
                            &file_path,
                            &path_buf,
                            matcher,
                            max_file_size,
                        ).await;
                        (file_path, result)
                    }
                })
                .collect();

            let batch_results = join_all(futures).await;
            for (file_path, result) in batch_results {
                if let Ok(node) = result {
                    file_nodes.insert(file_path, node);
                }
            }
        }
        
        let processing_duration = processing_start.elapsed();
        log::info!("Lazy metadata processing completed in {:.3}s", 
                  processing_duration.as_secs_f64());

        Self::build_lazy_directory_tree(path, &file_nodes, &file_map)
    }

    async fn process_file_lazy<P: AsRef<Path>>(
        file_path: P,
        root_path: P,
        matcher: &PatternMatcher,
        max_file_size: u64,
    ) -> Result<FileNodeLazy> {
        let file_path = file_path.as_ref();
        let root_path = root_path.as_ref();
        
        let metadata = fs::metadata(file_path).await?;
        let name = file_path
            .file_name()
            .unwrap_or_else(|| file_path.as_os_str())
            .to_string_lossy()
            .into_owned();

        let relative_path = file_path
            .strip_prefix(root_path)
            .unwrap_or(file_path)
            .to_string_lossy()
            .into_owned();

        let has_content = metadata.len() <= max_file_size 
            && PatternService::should_include_file(matcher, file_path)?
            && !is_binary_file(file_path);

        Ok(FileNodeLazy {
            name,
            path: file_path.to_path_buf(),
            relative_path,
            node_type: FileNodeType::File,
            size: metadata.len(),
            has_content,
            children: Vec::new(),
            depth: 0,
        })
    }


    fn build_lazy_directory_tree<P: AsRef<Path>>(
        current_path: P,
        file_nodes: &HashMap<PathBuf, FileNodeLazy>,
        file_map: &HashMap<PathBuf, Vec<PathBuf>>,
    ) -> Result<FileNodeLazy> {
        let current_path = current_path.as_ref();
        let name = current_path
            .file_name()
            .unwrap_or_else(|| current_path.as_os_str())
            .to_string_lossy()
            .into_owned();

        let mut children = Vec::new();
        let mut subdirectories = std::collections::HashSet::new();
        
        if let Some(child_paths) = file_map.get(current_path) {
            for child_path in child_paths {
                if child_path.is_file() {
                    if let Some(child_node) = file_nodes.get(child_path) {
                        children.push(child_node.clone());
                    }
                } else if child_path.is_dir() {
                    subdirectories.insert(child_path.clone());
                }
            }
        }

        for (dir_path, _) in file_map {
            if let Some(parent) = dir_path.parent() {
                if parent == current_path && dir_path.is_dir() {
                    subdirectories.insert(dir_path.clone());
                }
            }
        }

        for subdir_path in subdirectories {
            let subdir_node = Self::build_lazy_directory_tree(
                &subdir_path,
                file_nodes,
                file_map,
            )?;
            children.push(subdir_node);
        }

        children.sort_by(|a, b| {
            match (a.node_type, b.node_type) {
                (FileNodeType::Directory, FileNodeType::File) => std::cmp::Ordering::Less,
                (FileNodeType::File, FileNodeType::Directory) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        Ok(FileNodeLazy {
            name,
            path: current_path.to_path_buf(),
            relative_path: String::new(),
            node_type: FileNodeType::Directory,
            size: 0,
            has_content: false,
            children,
            depth: 0,
        })
    }

    pub fn write_content_to_file<P: AsRef<Path>>(node: &FileNodeLazy, output_path: P) -> Result<()> {
        let mut file = std::fs::File::create(output_path)?;
        node.write_content(&mut file).map_err(|e| GitingestError::FileSystemError(e.to_string()))?;
        Ok(())
    }

    pub fn generate_tree_string_lazy(node: &FileNodeLazy, prefix: &str, is_last: bool) -> String {
        let mut result = String::new();
        
        let connector = if is_last { "└── " } else { "├── " };
        let name_display = match node.node_type {
            FileNodeType::Directory => format!("{}/", node.name),
            FileNodeType::Symlink => format!("{} -> ?", node.name),
            FileNodeType::File => node.name.clone(),
        };
        
        result.push_str(&format!("{}{}{}\n", prefix, connector, name_display));
        
        if node.node_type == FileNodeType::Directory {
            let new_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
            
            for (i, child) in node.children.iter().enumerate() {
                let is_child_last = i == node.children.len() - 1;
                result.push_str(&Self::generate_tree_string_lazy(child, &new_prefix, is_child_last));
            }
        }
        
        result
    }
}

pub fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_file_size_formatting() {
        assert_eq!(format_file_size(500), "500 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1048576), "1.0 MB");
    }

    #[test]
    fn test_file_reading() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Hello, World!").unwrap();
        
        let content = FileService::read_file_content(&file_path)?;
        assert_eq!(content, "Hello, World!");
        
        Ok(())
    }
}