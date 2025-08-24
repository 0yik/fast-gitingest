"""Tests for fast-gitingest Python bindings."""

import pytest
from fast_gitingest import Gitingest, GitingestConfig, ingest_repo_sync


def test_gitingest_config_creation():
    """Test GitingestConfig can be created."""
    config = GitingestConfig(max_file_size=1024, max_files=100)
    assert config is not None


def test_gitingest_creation():
    """Test Gitingest can be created."""
    gitingest = Gitingest()
    assert gitingest is not None


def test_gitingest_with_config():
    """Test Gitingest can be created with config."""
    config = GitingestConfig(max_file_size=1024)
    gitingest = Gitingest(config=config)
    assert gitingest is not None


@pytest.mark.asyncio
async def test_ingest_invalid_repo():
    """Test ingesting an invalid repository."""
    gitingest = Gitingest()
    
    with pytest.raises(Exception):
        await gitingest.ingest("invalid://not-a-repo")


def test_sync_wrapper():
    """Test the synchronous wrapper function."""
    # This will fail with invalid URL, but we're testing the wrapper works
    try:
        ingest_repo_sync("invalid://not-a-repo")
    except Exception:
        pass  # Expected to fail, we're just testing the sync wrapper exists


def test_config_from_env():
    """Test config creation from environment."""
    config = GitingestConfig.from_env()
    assert config is not None