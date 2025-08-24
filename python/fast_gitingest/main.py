"""Command-line interface for fast-gitingest."""

import argparse
import asyncio
import json
import sys
from pathlib import Path
from typing import Optional

from .gitingest_python import Gitingest, GitingestConfig


def create_parser() -> argparse.ArgumentParser:
    """Create the argument parser for the CLI."""
    parser = argparse.ArgumentParser(
        prog="fast-gitingest",
        description="Fast Git repository ingestion and analysis tool"
    )
    
    subparsers = parser.add_subparsers(dest="command", help="Available commands")
    
    # Ingest command
    ingest_parser = subparsers.add_parser("ingest", help="Ingest and analyze a Git repository")
    ingest_parser.add_argument("input", help="Git repository URL or path")
    ingest_parser.add_argument(
        "-f", "--format", 
        choices=["text", "json", "markdown"], 
        default="text",
        help="Output format"
    )
    ingest_parser.add_argument("-o", "--output", help="Output file path")
    ingest_parser.add_argument("--include", help="Include patterns (comma-separated)")
    ingest_parser.add_argument("--exclude", help="Exclude patterns (comma-separated)")
    ingest_parser.add_argument("--max-file-size", type=int, help="Maximum file size in bytes")
    ingest_parser.add_argument("--max-files", type=int, help="Maximum number of files")
    ingest_parser.add_argument("--token", help="Authentication token")
    ingest_parser.add_argument("--branch", help="Specific branch to analyze")
    ingest_parser.add_argument("-v", "--verbose", action="store_true", help="Verbose output")
    
    # Platforms command
    subparsers.add_parser("platforms", help="Show supported platforms and features")
    
    # Config command
    subparsers.add_parser("config", help="Show configuration information")
    
    return parser


async def handle_ingest(args: argparse.Namespace) -> None:
    """Handle the ingest command."""
    include_patterns = None
    if args.include:
        include_patterns = [p.strip() for p in args.include.split(",")]
    
    exclude_patterns = None
    if args.exclude:
        exclude_patterns = [p.strip() for p in args.exclude.split(",")]
    
    try:
        gitingest = Gitingest()
        result = await gitingest.ingest(
            args.input,
            format=args.format,
            include_patterns=include_patterns,
            exclude_patterns=exclude_patterns,
            max_file_size=args.max_file_size,
            max_files=args.max_files,
            token=args.token,
            branch=args.branch,
        )
        
        # Format output based on requested format
        if args.format == "json":
            content = json.dumps(result, indent=2)
        elif args.format == "markdown":
            content = f"# Repository: {result['short_repo_url']}\n\n"
            content += f"## Summary\n{result['summary']}\n\n"
            content += f"## Directory Structure\n```\n{result['tree']}\n```\n\n"
            content += f"## File Contents\n{result['content']}"
        else:  # text
            content = f"Repository: {result['short_repo_url']}\n"
            content += f"Summary:\n{result['summary']}\n\n"
            content += f"Directory Structure:\n{result['tree']}\n\n"
            content += f"File Contents:\n{result['content']}"
        
        # Output to file or stdout
        if args.output:
            output_path = Path(args.output)
            output_path.write_text(content, encoding="utf-8")
            print(f"✅ Output written to: {output_path}")
        else:
            print(content)
            
    except Exception as e:
        print(f"❌ Error: {e}", file=sys.stderr)
        sys.exit(1)


def handle_platforms() -> None:
    """Handle the platforms command."""
    print("📊 Supported Git Hosting Platforms:")
    print("  • github.com")
    print("  • gitlab.com") 
    print("  • bitbucket.org")
    print()
    print("🔧 Features:")
    print("  • Repository cloning")
    print("  • Pattern matching")
    print("  • Gitignore support")
    print("  • File filtering")
    print("  • Tree generation")
    print("  • Content extraction")


def handle_config() -> None:
    """Handle the config command."""
    try:
        _ = GitingestConfig.from_env()
        print("⚙️  Current Configuration:")
        print("  Configuration loaded from environment variables")
        print("  See GitingestConfig class for available options")
    except Exception as e:
        print(f"❌ Error loading config: {e}", file=sys.stderr)


async def async_main() -> None:
    """Async main function."""
    parser = create_parser()
    args = parser.parse_args()
    
    if not args.command:
        parser.print_help()
        return
    
    if args.command == "ingest":
        await handle_ingest(args)
    elif args.command == "platforms":
        handle_platforms()
    elif args.command == "config":
        handle_config()


def main() -> None:
    """Main entry point for the CLI."""
    asyncio.run(async_main())


if __name__ == "__main__":
    main()