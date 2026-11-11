from __future__ import annotations

import argparse
import subprocess
import sys
from typing import Final

import pytest

from tasks.interpreters import minor_version, resolve

_MINOR: Final[int] = sys.version_info.minor


@pytest.mark.parametrize(
    "text",
    [
        pytest.param("3", id="major-only"),
        pytest.param("2.7", id="python-2"),
        pytest.param("3.x", id="non-numeric-minor"),
        pytest.param("3.12.1", id="patch-included"),
    ],
)
def test_minor_version_rejects_anything_but_3_x(text: str) -> None:
    with pytest.raises(argparse.ArgumentTypeError, match=r"expected 3\.X"):
        minor_version(text)


def test_resolve_finds_an_interpreter_of_the_version() -> None:
    interpreter = resolve(_MINOR)
    reported = subprocess.run(
        [interpreter.path, "-c", "import sys; print(sys.version_info.minor)"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout

    assert (interpreter.minor, int(reported)) == (_MINOR, _MINOR)


def test_resolve_rejects_a_version_uv_cannot_find() -> None:
    with pytest.raises(ValueError, match=r"no CPython 3\.99"):
        resolve(99)
