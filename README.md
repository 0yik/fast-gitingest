# Fast Git Ingest

A fast Git repository ingestion and analysis tool built with Rust, available as a CLI tool.

## Architecture

This project uses a Cargo workspace with two main components:

- **`gitingest/`** - Core library containing all business logic
- **`gitingest-cli/`** - Command-line interface tool

## Quick Start

### CLI Usage

```bash
# Build all components
cargo build

# Ingest a repository
cargo run -p gitingest-cli -- ingest https://github.com/user/repo

# Show supported platforms
cargo run -p gitingest-cli -- platforms

# Show configuration
cargo run -p gitingest-cli -- config

# Get help
cargo run -p gitingest-cli -- --help
```



## Configuration

Environment variables:

- `MAX_FILE_SIZE` - Maximum file size in bytes
- `MAX_FILES` - Maximum number of files to process
- `GITHUB_TOKEN` - GitHub personal access token

## Development

```bash
# Build specific component
cargo build -p gitingest
cargo build -p gitingest-cli

# Run tests
cargo test

# Check all components
cargo check
```

## Features

- ✅ Repository cloning and analysis
- ✅ Pattern matching and gitignore support
- ✅ File filtering and content extraction
- ✅ Multiple output formats (JSON, Text, Markdown)
- ✅ CLI interface
- ✅ Concurrent file processing
- ✅ Memory-efficient streaming