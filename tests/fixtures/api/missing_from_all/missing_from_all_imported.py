import sys
from os.path import join  # noqa: F401
from pathlib import Path


def run() -> Path:
    return Path(sys.argv[0])


__all__ = ["run"]
