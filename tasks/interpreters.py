"""Find the CPython interpreters the maintenance tasks compile code under."""

from __future__ import annotations

import argparse
import re
import subprocess
from dataclasses import dataclass
from typing import Final

_MINOR_VERSION_PATTERN: Final[re.Pattern[str]] = re.compile(r"3\.(\d+)")


def minor_version(text: str) -> int:
    if match := _MINOR_VERSION_PATTERN.fullmatch(text):
        return int(match.group(1))
    msg = f"expected 3.X, got {text!r}"
    raise argparse.ArgumentTypeError(msg)


def resolve(minor: int) -> Interpreter:
    # Example targets can be older than the interpreter needed to install simplint.
    found = subprocess.run(
        ["uv", "python", "find", "--no-project", f"3.{minor}"], capture_output=True, text=True, check=False
    )
    if found.returncode:
        msg = f"no CPython 3.{minor} for the examples: {found.stderr.strip()}"
        raise ValueError(msg)
    return Interpreter(minor=minor, path=found.stdout.strip())


@dataclass(frozen=True, slots=True, kw_only=True)
class Interpreter:
    minor: int
    path: str


__all__ = [
    "Interpreter",
    "minor_version",
    "resolve",
]
