"""Fast Git repository ingestion and analysis tool.

This package provides Python bindings for a high-performance Rust-based
Git repository analysis tool.
"""

__version__ = "0.1.0"

from .gitingest_python import Gitingest, GitingestConfig, cli

__all__ = [
    "Gitingest",
    "GitingestConfig", 
    "cli",
    "ingest_repo",
    "ingest_repo_sync",
    "main",
]

# Import main function separately to avoid circular imports
def _import_main():
    from .main import main
    return main

main = _import_main()

# Convenience functions for common use cases
async def ingest_repo(
    repo_url: str,
    format: str = "text",
    include_patterns: list[str] | None = None,
    exclude_patterns: list[str] | None = None,
    max_file_size: int | None = None,
    max_files: int | None = None,
    token: str | None = None,
    branch: str | None = None,
) -> dict:
    """
    Ingest a Git repository and return the analysis results.
    
    Args:
        repo_url: Git repository URL or local path
        format: Output format ('text', 'json', or 'markdown')
        include_patterns: List of patterns to include
        exclude_patterns: List of patterns to exclude
        max_file_size: Maximum file size in bytes
        max_files: Maximum number of files to process
        token: Authentication token for private repositories
        branch: Specific branch to analyze
        
    Returns:
        Dictionary containing repository analysis results
        
    Example:
        >>> import asyncio
        >>> from fast_gitingest import ingest_repo
        >>> 
        >>> async def main():
        ...     result = await ingest_repo("https://github.com/user/repo")
        ...     print(result["summary"])
        >>> 
        >>> asyncio.run(main())
    """
    gitingest = Gitingest()
    return await gitingest.ingest(
        repo_url,
        format=format,
        include_patterns=include_patterns,
        exclude_patterns=exclude_patterns,
        max_file_size=max_file_size,
        max_files=max_files,
        token=token,
        branch=branch,
    )


def ingest_repo_sync(
    repo_url: str,
    format: str = "text", 
    include_patterns: list[str] | None = None,
    exclude_patterns: list[str] | None = None,
    max_file_size: int | None = None,
    max_files: int | None = None,
    token: str | None = None,
    branch: str | None = None,
) -> dict:
    """
    Synchronous wrapper for ingest_repo.
    
    Args:
        repo_url: Git repository URL or local path
        format: Output format ('text', 'json', or 'markdown')
        include_patterns: List of patterns to include
        exclude_patterns: List of patterns to exclude
        max_file_size: Maximum file size in bytes
        max_files: Maximum number of files to process
        token: Authentication token for private repositories
        branch: Specific branch to analyze
        
    Returns:
        Dictionary containing repository analysis results
        
    Example:
        >>> from fast_gitingest import ingest_repo_sync
        >>> result = ingest_repo_sync("https://github.com/user/repo")
        >>> print(result["summary"])
    """
    import asyncio
    return asyncio.run(ingest_repo(
        repo_url,
        format=format,
        include_patterns=include_patterns,
        exclude_patterns=exclude_patterns,
        max_file_size=max_file_size,
        max_files=max_files,
        token=token,
        branch=branch,
    ))