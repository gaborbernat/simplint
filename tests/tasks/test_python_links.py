from __future__ import annotations

import io
import zlib
from typing import TYPE_CHECKING

import pytest

from tasks.python_links import Inventory, main, relink

if TYPE_CHECKING:
    from pathlib import Path

    from pytest_mock import MockerFixture

_ENTRIES = """\
contextlib.chdir py:class 1 library/contextlib.html#$ -
os.chdir py:function 1 library/os.html#$ -
open py:function 1 builtins/functions.html#$ -
TypeError py:exception 1 builtins/exceptions.html#$ -
dict py:class 1 builtins/stdtypes.html#$ -
"""


def _inventory_bytes() -> bytes:
    header = b"# Sphinx inventory version 2\n# Project: Python\n# Version: 3.15\n# The remainder is zlib-compressed.\n"
    return header + zlib.compress(_ENTRIES.encode())


@pytest.fixture
def inventory() -> Inventory:
    return Inventory(_inventory_bytes())


@pytest.mark.parametrize(
    ("text", "expected"),
    [
        pytest.param(
            "Use `contextlib.chdir`, not `contextlib.chdir` twice.",
            "Use [`contextlib.chdir`](https://docs.python.org/3/library/contextlib.html#contextlib.chdir), not"
            " `contextlib.chdir` twice.",
            id="first_dotted_mention",
        ),
        pytest.param(
            "Call `os.chdir(target)` or `open()`.",
            "Call [`os.chdir(target)`](https://docs.python.org/3/library/os.html#os.chdir) or"
            " [`open()`](https://docs.python.org/3/builtins/functions.html#open).",
            id="calls",
        ),
        pytest.param(
            "Raises `TypeError` from a `dict` and `open`.",
            "Raises [`TypeError`](https://docs.python.org/3/builtins/exceptions.html#TypeError) from a `dict` and"
            " `open`.",
            id="undotted_names_only_exceptions_and_calls",
        ),
        pytest.param(
            "A [`dict`](https://docs.python.org/3/library/stdtypes.html#dict) here.",
            "A [`dict`](https://docs.python.org/3/builtins/stdtypes.html#dict) here.",
            id="moved_page",
        ),
        pytest.param(
            "See [the guide](https://docs.python.org/3/library/os.html#files-and-directories).",
            "See [the guide](https://docs.python.org/3/library/os.html#files-and-directories).",
            id="section_of_known_page",
        ),
        pytest.param(
            "Already [`os.chdir`](https://docs.python.org/3/library/os.html#os.chdir), so `os.chdir` stays.",
            "Already [`os.chdir`](https://docs.python.org/3/library/os.html#os.chdir), so `os.chdir` stays.",
            id="already_linked",
        ),
        pytest.param(
            "`type(value) is os.chdir` is one span, `pkg.missing` unknown.",
            "`type(value) is os.chdir` is one span, `pkg.missing` unknown.",
            id="spans_and_unknown_names",
        ),
        pytest.param(
            "# `os.chdir`\n\n```python\nos.chdir\n`os.chdir`\n```",
            "# `os.chdir`\n\n```python\nos.chdir\n`os.chdir`\n```",
            id="headings_and_fences",
        ),
    ],
)
def test_relink(inventory: Inventory, text: str, expected: str) -> None:
    assert relink(text, inventory) == (expected, [])


def test_relink_reports_links_the_inventory_lacks(inventory: Inventory) -> None:
    text = "[gone](https://docs.python.org/3/library/gone.html#gone)"

    assert relink(text, inventory) == (text, ["https://docs.python.org/3/library/gone.html#gone"])


@pytest.fixture
def served(mocker: MockerFixture) -> None:
    mocker.patch("tasks.python_links.urllib.request.urlopen", side_effect=lambda _: io.BytesIO(_inventory_bytes()))


@pytest.mark.usefixtures("served")
def test_main_reports_missing_links_without_fix(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    page = tmp_path / "rules" / "page.md"
    page.parent.mkdir()
    page.write_text("Use `os.chdir`.\n", encoding="utf-8")

    assert main([str(tmp_path / "rules")]) == 1
    assert page.read_text(encoding="utf-8") == "Use `os.chdir`.\n"
    assert "links are missing or stale" in capsys.readouterr().err


@pytest.mark.usefixtures("served")
def test_main_fixes_files(tmp_path: Path) -> None:
    page = tmp_path / "page.md"
    page.write_text("Use `os.chdir`.\n", encoding="utf-8")

    assert main(["--fix", str(page)]) == 0
    assert page.read_text(encoding="utf-8") == (
        "Use [`os.chdir`](https://docs.python.org/3/library/os.html#os.chdir).\n"
    )


@pytest.mark.usefixtures("served")
def test_main_passes_linked_files_and_reports_unknown_links(tmp_path: Path) -> None:
    linked = tmp_path / "linked.md"
    linked.write_text("Use [`os.chdir`](https://docs.python.org/3/library/os.html#os.chdir).\n", encoding="utf-8")
    unknown = tmp_path / "unknown.md"
    unknown.write_text("[gone](https://docs.python.org/3/library/gone.html)\n", encoding="utf-8")

    assert main([str(linked)]) == 0
    assert main([str(unknown)]) == 1
