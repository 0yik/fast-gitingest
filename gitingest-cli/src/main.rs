use clap::{Parser, ValueEnum};
use gitingest::{AppConfig, IngestService, IngestRequest, DownloadFormat, UrlParser};
use std::path::PathBuf;
use anyhow::Result;

#[derive(Parser)]
#[command(name = "gitingest")]
#[command(about = "A fast Git repository ingestion and analysis tool")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
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
    
    #[arg(short, long, help = "Enable verbose logging")]
    verbose: bool,
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
    
    // Main repository ingestion logic
    let mut request = IngestRequest {
        input_text: cli.input.clone(),
        download_format: Some(cli.format.into()),
        include_patterns: cli.include.map(|s| s.split(',').map(|s| s.trim().to_string()).collect()),
        exclude_patterns: cli.exclude.map(|s| s.split(',').map(|s| s.trim().to_string()).collect()),
        max_file_size: cli.max_file_size,
        max_files: cli.max_files,
        pattern_type: None,
        pattern: None,
        token: None,
        branch: None,
        include_submodules: None,
    };
    
    // Generate automatic filename if no output is specified
    let output_path = if let Some(output_path) = cli.output {
        // If output filename is provided but doesn't match the format, adjust format based on extension
        if let Some(ext) = output_path.extension().and_then(|e| e.to_str()) {
            let format_from_ext = match ext {
                "txt" => DownloadFormat::Text,
                "md" => DownloadFormat::Markdown,
                "json" => DownloadFormat::Json,
                _ => cli.format.into(),
            };
            request.download_format = Some(format_from_ext);
        }
        output_path
    } else {
        // Parse repository URL to extract name for automatic filename
        match UrlParser::parse_git_url(&cli.input) {
            Ok(repo) => {
                let extension = match cli.format {
                    OutputFormat::Text => "txt",
                    OutputFormat::Markdown => "md", 
                    OutputFormat::Json => "json",
                };
                PathBuf::from(format!("{}.{}", repo.name, extension))
            },
            Err(_) => {
                // Fallback to generic name if URL parsing fails
                let extension = match cli.format {
                    OutputFormat::Text => "txt",
                    OutputFormat::Markdown => "md",
                    OutputFormat::Json => "json", 
                };
                PathBuf::from(format!("output.{}", extension))
            }
        }
    };
    
    tracing::info!("Starting ingestion of: {}", cli.input);
    
    match IngestService::process_repository(request.clone(), &config).await {
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
    
    Ok(())
}