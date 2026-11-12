"""Find the CPython interpreters the maintenance tasks compile code under."""

from __future__ import annotations

import argparse
import re
import subprocess
from dataclasses import dataclass
from typing import Final


@dataclass(frozen=True, slots=True, kw_only=True)
class Interpreter:
    # A CPython of a minor version, found by uv.
    minor: int
    path: str


_MINOR_VERSION_PATTERN: Final[re.Pattern[str]] = re.compile(r"3\.(\d+)")


def minor_version(text: str) -> int:
    # Read a CPython version such as 3.12 as its minor number.
    #
    # :param text: the version, of the form 3.X
    # :returns: the minor number
    # :raises argparse.ArgumentTypeError: if the text is not a 3.X version
    if match := _MINOR_VERSION_PATTERN.fullmatch(text):
        return int(match.group(1))
    msg = f"expected 3.X, got {text!r}"
    raise argparse.ArgumentTypeError(msg)


def resolve(minor: int) -> Interpreter:
    # Find the CPython of a minor version with uv, which the CI job installs ahead of this task.
    #
    # The project's own requires-python is skipped, as it would warn about the older versions this task exists to test.
    #
    # :param minor: the minor number
    # :returns: the interpreter
    # :raises ValueError: if uv finds no interpreter of that version
    found = subprocess.run(
        ["uv", "python", "find", "--no-project", f"3.{minor}"], capture_output=True, text=True, check=False
    )
    if found.returncode:
        msg = f"no CPython 3.{minor} for the examples: {found.stderr.strip()}"
        raise ValueError(msg)
    return Interpreter(minor=minor, path=found.stdout.strip())


__all__ = [
    "Interpreter",
    "minor_version",
    "resolve",
]
