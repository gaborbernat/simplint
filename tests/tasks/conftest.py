from __future__ import annotations

import time
from types import SimpleNamespace
from typing import TYPE_CHECKING, Final

import pytest

from tasks import download as download_module

if TYPE_CHECKING:
    from pathlib import Path

    from pytest_mock import MockerFixture, MockType

_REGISTRY: Final[str] = """\
rules! {
    AllFormat => ("SMP405", "all-format", "`__all__` format", Always, api::all_format { module }),
}

pub const GROUPS: [(&str, &str, &str); 10] = [
    ("comments", "Comments", "Comments"),
    ("naming", "Naming", "Naming"),
    ("layout", "Module layout", "Module layout"),
    ("imports", "Imports and dependencies", "Imports"),
    ("api", "Public API", "Public API"),
    ("logic", "Logic", "Logic"),
    ("typing", "Typing", "Typing"),
    ("classes", "Classes", "Classes"),
    ("testing", "Tests", "Tests"),
    ("docstrings", "Docstrings", "Docstrings"),
];
"""


@pytest.fixture
def registry(tmp_path: Path) -> Path:
    # the one-rule registry the rule list reads, so a test names a rule the list really has
    (tmp_path / "src" / "lint").mkdir(parents=True)
    path = tmp_path / "src" / "lint" / "registry.rs"
    path.write_text(_REGISTRY, encoding="utf-8")
    return path


@pytest.fixture
def sleep(mocker: MockerFixture) -> MockType:
    # The module's own clock, so the patch does not reach other threads that call the shared `time.sleep`.
    sleep = mocker.create_autospec(time.sleep)
    mocker.patch.object(download_module, "time", SimpleNamespace(sleep=sleep))
    return sleep
