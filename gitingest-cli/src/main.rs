use clap::{Parser, Subcommand, ValueEnum};
use gitingest::{AppConfig, IngestService, IngestRequest, DownloadFormat, UrlParser};
use std::path::PathBuf;
use uuid::Uuid;
use anyhow::Result;

#[derive(Parser)]
#[command(name = "gitingest")]
#[command(about = "A fast Git repository ingestion and analysis tool")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Ingest and analyze a Git repository")]
    Ingest {
        #[arg(help = "Git repository URL or path")]
        input: String,
        
        #[arg(short, long, value_enum, default_value = "text", help = "Output format")]
        format: OutputFormat,
        
        #[arg(short, long, help = "Output file path")]
        output: Option<PathBuf>,
        
        #[arg(long, help = "Include patterns (comma-separated)")]
        include: Option<String>,
        
        #[arg(long, help = "Exclude patterns (comma-separated)")]
        exclude: Option<String>,
        
        #[arg(long, help = "Maximum file size in bytes")]
        max_file_size: Option<u64>,
        
        #[arg(long, help = "Maximum number of files")]
        max_files: Option<usize>,
    },
    
    #[command(about = "Show supported platforms and features")]
    Platforms,
    
    #[command(about = "Show configuration information")]
    Config,
}

#[derive(Clone, Copy, ValueEnum)]
enum OutputFormat {
    Json,
    Text,
    Markdown,
}

impl From<OutputFormat> for DownloadFormat {
    fn from(format: OutputFormat) -> Self {
        match format {
            OutputFormat::Json => DownloadFormat::Json,
            OutputFormat::Text => DownloadFormat::Text,
            OutputFormat::Markdown => DownloadFormat::Markdown,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(format!("gitingest={},gitingest_cli={}", log_level, log_level))
        .init();
    
    dotenv::dotenv().ok();
    let config = AppConfig::from_env()?;
    
    match cli.command {
        Commands::Ingest {
            input,
            format,
            output,
            include,
            exclude,
            max_file_size,
            max_files,
        } => {
            let mut request = IngestRequest {
                input_text: input.clone(),
                download_format: Some(format.into()),
                include_patterns: include.map(|s| s.split(',').map(|s| s.trim().to_string()).collect()),
                exclude_patterns: exclude.map(|s| s.split(',').map(|s| s.trim().to_string()).collect()),
                max_file_size,
                max_files,
                pattern_type: None,
                pattern: None,
                token: None,
                branch: None,
                include_submodules: None,
            };
            
            // Generate automatic filename if no output is specified
            let output_path = if let Some(output_path) = output {
                // If output filename is provided but doesn't match the format, adjust format based on extension
                if let Some(ext) = output_path.extension().and_then(|e| e.to_str()) {
                    let format_from_ext = match ext {
                        "txt" => DownloadFormat::Text,
                        "md" => DownloadFormat::Markdown,
                        "json" => DownloadFormat::Json,
                        _ => format.into(),
                    };
                    request.download_format = Some(format_from_ext);
                }
                output_path
            } else {
                // Parse repository URL to extract name for automatic filename
                match UrlParser::parse_git_url(&input) {
                    Ok(repo) => {
                        let extension = match format {
                            OutputFormat::Text => "txt",
                            OutputFormat::Markdown => "md", 
                            OutputFormat::Json => "json",
                        };
                        PathBuf::from(format!("{}.{}", repo.name, extension))
                    },
                    Err(_) => {
                        // Fallback to generic name if URL parsing fails
                        let extension = match format {
                            OutputFormat::Text => "txt",
                            OutputFormat::Markdown => "md",
                            OutputFormat::Json => "json", 
                        };
                        PathBuf::from(format!("output.{}", extension))
                    }
                }
            };
            
            let id = Uuid::new_v4();
            tracing::info!("Starting ingestion of: {}", input);
            tracing::debug!("Processing with ID: {}", id);
            
            match IngestService::process_repository(request.clone(), &config, id).await {
                Ok(response) => {
                    let content = match request.download_format.unwrap_or(DownloadFormat::Text) {
                        DownloadFormat::Json => serde_json::to_string_pretty(&response)?,
                        DownloadFormat::Text => format!(
                            "Repository: {}\nSummary:\n{}\n\nDirectory Structure:\n{}\n\nFile Contents:\n{}",
                            response.short_repo_url,
                            response.summary,
                            response.tree,
                            response.content
                        ),
                        DownloadFormat::Markdown => format!(
                            "# Repository: {}\n\n## Summary\n{}\n\n## Directory Structure\n```\n{}\n```\n\n## File Contents\n{}",
                            response.short_repo_url,
                            response.summary,
                            response.tree,
                            response.content
                        ),
                    };
                    
                    std::fs::write(&output_path, content)?;
                    println!("✅ Output written to: {}", output_path.display());
                    
                    tracing::info!("✅ Ingestion completed successfully");
                },
                Err(err) => {
                    tracing::error!("❌ Ingestion failed: {:?}", err);
                    std::process::exit(1);
                }
            }
        },
        
        Commands::Platforms => {
            println!("📊 Supported Git Hosting Platforms:");
            println!("  • github.com");
            println!("  • gitlab.com");
            println!("  • bitbucket.org");
            println!();
            println!("🔧 Features:");
            println!("  • Repository cloning");
            println!("  • Pattern matching");
            println!("  • Gitignore support");
            println!("  • File filtering");
            println!("  • Tree generation");
            println!("  • Content extraction");
        },
        
        Commands::Config => {
            println!("⚙️  Current Configuration:");
            println!("  Max file size: {} bytes", config.max_file_size);
            println!("  Max files: {}", config.max_files);
            println!("  Max total size: {} bytes", config.max_total_size);
            println!("  Max directory depth: {}", config.max_directory_depth);
            println!("  Default timeout: {}s", config.default_timeout);
            
            if std::env::var("GITHUB_TOKEN").is_ok() {
                println!("  GitHub token: ✅ Configured");
            } else {
                println!("  GitHub token: ❌ Not configured");
            }
        }
    }
    
    Ok(())
}