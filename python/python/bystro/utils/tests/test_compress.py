import pytest
import shutil
from bystro.utils.compress import _get_gzip_program_path


def test_bgzip_found(monkeypatch):
    monkeypatch.setattr(
        shutil, "which", lambda x: "/path/to/bgzip" if x == "bgzip" else None
    )
    assert _get_gzip_program_path() == "/path/to/bgzip"


def test_bgzip_not_found_gzip_found(monkeypatch):
    monkeypatch.setattr(
        shutil, "which", lambda x: "/path/to/gzip" if x == "gzip" else None
    )
    assert _get_gzip_program_path() == "/path/to/gzip"


def test_neither_found(monkeypatch):
    monkeypatch.setattr(shutil, "which", lambda _: None)
    with pytest.raises(OSError, match="Neither gzip nor bgzip not found on system"):
        _get_gzip_program_path()
