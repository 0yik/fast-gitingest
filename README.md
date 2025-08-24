# ⚡ Fast GitIngest

> **Blazingly fast Git repository ingestion tool that transforms any repository into LLM-ready content**

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg?style=flat-square)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=flat-square)](https://opensource.org/licenses/MIT)
[![Performance](https://img.shields.io/badge/speed-18.5x%20faster-brightgreen.svg?style=flat-square)](#benchmark)

Transform any Git repository into structured, AI-ready content with **lightning-fast performance**. Built in Rust for maximum speed and efficiency.

---

## 🤖 Inspired By

This project reimagines the Python [gitingest](https://github.com/coderamp-labs/gitingest) with enterprise-grade performance:

### Why We Built This
- **🚀 Dramatic Speed Improvements**: 18.5x faster processing with real benchmarks
- **⚡ True Concurrency**: Tokio async runtime vs Python's limited threading
- **🧠 Memory Efficiency**: Streaming architecture vs loading everything in RAM
- **🛠️ Production Ready**: Enterprise error handling and configuration management

### Architecture Advantages
- **Compiled Performance**: Native binary vs interpreted Python execution
- **Memory Safety**: Rust's ownership system prevents common memory errors
- **Async by Design**: Built-in concurrent processing capabilities
- **Zero Runtime Dependencies**: Single binary deployment

---

## 🚀 Real-World Benchmark

**Head-to-head comparison on Kubernetes repository** - the largest, most complex Go project:

### Fast GitIngest (Rust) - Text Format
```
Repository: kubernetes/kubernetes
├─ Total Time:         26.40 seconds ⚡
├─ Files Processed:    27,621 files
├─ Memory Usage:       319 MB peak
├─ Output Size:        141 MB (text)
├─ Processing Speed:   1,046 files/sec
└─ CPU Usage:          67% efficient
```

### Python coderamp-labs/gitingest - Text Format
```
Repository: kubernetes/kubernetes  
├─ Total Time:         487.58 seconds (8:07) 🐌
├─ Files Processed:    10,002 files (hit limit)
├─ Memory Usage:       817 MB peak
├─ Output Size:        64 MB (text)
├─ Processing Speed:   20.5 files/sec
└─ CPU Usage:          96% maxed out
```

### 📊 Performance Improvements

| Metric | Fast GitIngest | Python gitingest | Improvement |
|--------|----------------|-----------------|------------|
| **Processing Time** | 26.40s | 487.58s | **18.5x faster** |
| **Files Processed** | 27,621 | 10,002 | **2.8x more files** |
| **Memory Usage** | 319 MB | 817 MB | **2.6x less memory** |
| **Speed per File** | 1,046 files/sec | 20.5 files/sec | **51x faster** |
| **CPU Efficiency** | 67% | 96% | **More efficient** |

**The Rust version processed nearly 3x more files in 18.5x less time while using 2.6x less memory!**

---

## 💪 Why Fast GitIngest?

### 🔥 **Performance Optimized**
- **Concurrent Processing**: 1,000 parallel file operations by default
- **Batch Processing**: Smart chunking (500 files/batch) prevents memory overload
- **Streaming Architecture**: Process repositories of any size without RAM limits  
- **Shallow Cloning**: Skip Git history, get code instantly (depth=1)

### 🛡️ **Enterprise Ready**
- **Pattern Matching**: Advanced gitignore support with include/exclude rules
- **Security First**: Private repository support with GitHub tokens
- **Format Flexibility**: JSON, Markdown, Plain Text output
- **Size Controls**: Configurable limits and intelligent filtering

### 🎯 **Perfect For**
- **AI/LLM Engineers** - Get repository context for code analysis
- **DevOps Teams** - Repository auditing and documentation  
- **Code Analysis** - Extract and analyze codebase structure
- **Documentation Generation** - Auto-generate project overviews
- **Security Audits** - Rapid codebase scanning

---

## 🚀 Quick Start

### Installation

```bash
git clone https://github.com/yourusername/fast-gitingest
cd fast-gitingest
cargo build --release
```

### Basic Usage

```bash
# Analyze any repository instantly
./target/release/gitingest https://github.com/user/awesome-project

# Specify output format and file
gitingest https://github.com/user/repo --format json -o analysis.json

# Include specific patterns only
gitingest https://github.com/user/repo --include "*.rs,*.toml" --format markdown

# Process with custom limits and verbose output
gitingest https://github.com/user/repo --max-files 50000 --verbose
```

---

## 🎨 Output Examples

**📝 Text Format (Default)**
```
Repository: kubernetes/kubernetes
Summary:
Repository: kubernetes/kubernetes
Files processed: 27621
Total size: 277.9 MB
Host: github.com

Directory Structure:
└── kubernetes/
    ├── .github/
    ├── api/
    ├── build/
    └── cmd/

File Contents:
// Structured file content here...
```

**📊 JSON Format**
```json
{
  "id": "uuid-here",
  "repo_url": "https://github.com/kubernetes/kubernetes",
  "short_repo_url": "kubernetes/kubernetes", 
  "summary": "Repository: kubernetes/kubernetes\nFiles processed: 27621...",
  "tree": "└── kubernetes/\n    ├── .github/\n...",
  "content": "// File contents here...",
  "status": "Completed"
}
```

**📝 Markdown Format**
```markdown
# Repository: kubernetes/kubernetes

## Summary
Repository: kubernetes/kubernetes
Files processed: 27621
Total size: 277.9 MB

## Directory Structure
```
└── kubernetes/
    ├── .github/
    └── cmd/
```

## File Contents
[Structured content with syntax highlighting]
```

---

## ⚙️ Advanced Configuration

### Performance Tuning

```bash
# High-performance mode for large repositories
export CONCURRENT_FILE_LIMIT=1000    # Parallel processing limit
export BATCH_SIZE=500                # Files per batch
export MAX_FILE_SIZE=10485760        # 10MB per file limit

# Memory-optimized mode for constrained environments
export CONCURRENT_FILE_LIMIT=100
export BATCH_SIZE=50
export MAX_FILES=5000
```

### Security & Access

```bash
# Private repository access
export GITHUB_TOKEN="ghp_your_token_here"

# Allowed Git hosting platforms  
export ALLOWED_HOSTS="github.com,gitlab.com,bitbucket.org"
```

### Processing Limits

```bash
# Repository size controls
export MAX_TOTAL_SIZE=524288000      # 500MB total limit
export MAX_DIRECTORY_DEPTH=20        # Recursion depth limit
export DEFAULT_TIMEOUT=120           # Processing timeout (seconds)
```

---

## 🏗️ Architecture

**Modular, high-performance design** built for scale:

```
┌─────────────────────┐    ┌──────────────────────┐
│   gitingest-cli/    │───▶│     gitingest/       │
│   • CLI Interface   │    │   • Core Engine      │
│   • Argument Parsing│    │   • Business Logic   │  
│   • Output Formatting    │   • Performance Opts │
└─────────────────────┘    └──────────────────────┘
                                        │
                           ┌────────────▼────────────┐
                           │       Services          │
                           │  ┌─────────────────────┐ │
                           │  │ • IngestService    │ │
                           │  │ • GitService       │ │
                           │  │ • FileService      │ │
                           │  │ • PatternService   │ │
                           │  └─────────────────────┘ │
                           └─────────────────────────┘
```

### Core Performance Features

- **🔧 GitService**: Lightning-fast shallow cloning with authentication
- **📁 FileService**: Concurrent file processing with semaphore-controlled threading
- **🎯 PatternService**: Advanced pattern matching and .gitignore support  
- **⚡ IngestService**: End-to-end pipeline orchestration with detailed metrics

---

## 💡 Use Cases

### AI & Machine Learning
```bash
# Prepare codebase for LLM analysis
gitingest https://github.com/company/backend-service --format json

# Extract specific file types for training data
gitingest https://github.com/org/ml-project --include "*.py,*.md,*.yml"
```

### Documentation & Analysis
```bash
# Generate project documentation
gitingest https://github.com/team/frontend --format markdown -o project-docs.md

# Security audit preparation
gitingest https://github.com/company/webapp --include "*.js,*.ts,*.json" --verbose
```

### DevOps & CI/CD
```bash
# Repository structure analysis
gitingest https://github.com/org/microservices --format json | jq '.tree'

# Code review preparation  
gitingest https://github.com/team/feature-branch --exclude "node_modules,dist,build"
```

---

## 🔧 Development

### Building from Source

```bash
# Clone and build
git clone https://github.com/yourusername/fast-gitingest
cd fast-gitingest
cargo build --release

# Run tests
cargo test

# Development build with debug symbols
cargo build

# Check compilation without building
cargo check
```

### Performance Testing

```bash
# Enable detailed performance logging
RUST_LOG=debug ./target/release/gitingest https://github.com/user/repo --verbose

# Benchmark with system metrics
gtime -v ./target/release/gitingest https://github.com/user/repo

# Compare formats
./target/release/gitingest https://github.com/user/repo --format text
./target/release/gitingest https://github.com/user/repo --format json  
./target/release/gitingest https://github.com/user/repo --format markdown
```

---

## 📈 Roadmap

- [ ] **Multi-format Output**: XML, YAML, TOML support
- [ ] **Plugin System**: Custom content processors
- [ ] **Cloud Integration**: S3, GCS direct upload
- [ ] **REST API**: HTTP service mode
- [ ] **Docker Images**: Containerized deployment
- [ ] **GitHub Actions**: CI/CD integration
- [ ] **Web Interface**: Browser-based analysis

---

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

---

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for:

- 🐛 Bug reports and fixes
- ✨ Feature requests and implementations  
- 📚 Documentation improvements
- 🧪 Performance optimizations
- 🔧 Platform support expansion

---

<div align="center">

**⭐ Star this repo if it helped you process repositories 18.5x faster!**

[Report Bug](https://github.com/0yik/fast-gitingest/issues) • 
[Request Feature](https://github.com/0yik/fast-gitingest/issues) •
[Documentation](https://github.com/0yik/fast-gitingest/wiki)

---

**Made with ⚡ by developers, for developers**

</div>