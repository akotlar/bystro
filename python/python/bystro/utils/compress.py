"""Utility functions for safely invoking tar in cross-OS manner."""
import shutil


def _get_gzip_program_path() -> str:
    bgzip = shutil.which("bgzip")

    if bgzip:
        return bgzip

    gzip = shutil.which("gzip")

    if gzip:
        return gzip

    raise OSError("Neither gzip nor bgzip not found on system")


GZIP_EXECUTABLE = _get_gzip_program_path()
