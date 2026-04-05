"""Tests for progress manager."""
from progress import ProgressManager


def test_create_empty_progress(temp_progress_file):
    """Test creating a new progress manager."""

    manager = ProgressManager(temp_progress_file)
    assert manager.state.phase == "discovery"
    assert manager.state.total_discovered == 0


def test_add_url(temp_progress_file):
    """Test adding a URL to progress."""

    manager = ProgressManager(temp_progress_file)
    manager.add_url("https://example.com/card/n1.html", "green_feeds", "Травы")

    assert manager.state.total_discovered == 1
    assert manager.state.discovered_urls[0].url == "https://example.com/card/n1.html"
    assert manager.state.discovered_urls[0].status == "pending"


def test_mark_completed(temp_progress_file):
    """Test marking a URL as completed."""

    manager = ProgressManager(temp_progress_file)
    manager.add_url("https://example.com/card/n1.html", "green_feeds", "Травы")
    manager.mark_completed("https://example.com/card/n1.html")

    assert manager.state.discovered_urls[0].status == "parsed"
    assert manager.state.total_parsed == 1


def test_mark_failed(temp_progress_file):
    """Test marking a URL as failed."""

    manager = ProgressManager(temp_progress_file)
    manager.add_url("https://example.com/card/n1.html", "green_feeds", "Травы")
    manager.mark_failed("https://example.com/card/n1.html", "Connection timeout")

    assert manager.state.discovered_urls[0].status == "failed"
    assert manager.state.discovered_urls[0].error_message == "Connection timeout"
    assert manager.state.total_failed == 1


def test_save_and_load(temp_progress_file):
    """Test saving and loading progress state."""

    manager = ProgressManager(temp_progress_file)
    manager.add_url("https://example.com/card/n1.html", "green_feeds", "Травы")
    manager.mark_completed("https://example.com/card/n1.html")
    manager.save()

    loaded = ProgressManager(temp_progress_file)
    assert loaded.state.total_discovered == 1
    assert loaded.state.total_parsed == 1
    assert loaded.state.discovered_urls[0].status == "parsed"


def test_get_pending_urls(temp_progress_file):
    """Test getting pending URLs."""

    manager = ProgressManager(temp_progress_file)
    manager.add_url("https://example.com/card/n1.html", "cat1", "Sub1")
    manager.add_url("https://example.com/card/n2.html", "cat1", "Sub1")
    manager.mark_completed("https://example.com/card/n1.html")

    pending = manager.get_pending_urls()
    assert len(pending) == 1
    assert pending[0].url == "https://example.com/card/n2.html"
