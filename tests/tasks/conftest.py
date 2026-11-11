from __future__ import annotations

import io
import tarfile
import time
from pathlib import Path
from types import SimpleNamespace
from typing import TYPE_CHECKING, Final

import pytest

from tasks import download as download_module

if TYPE_CHECKING:
    from collections.abc import Callable

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
def root() -> Path:
    return Path(__file__).resolve().parents[2]


@pytest.fixture
def files_url() -> str:
    # The address PyPI serves source archives under, the prefix of the pinned URLs a test names.
    return "https://files.pythonhosted.org/packages/ab/cd"


@pytest.fixture
def tar_gz() -> Callable[[dict[str, bytes], dict[str, str] | None], bytes]:
    # A gzipped tar of regular files, by member name and content, and of symbolic links, by member name and target.
    def build(members: dict[str, bytes], links: dict[str, str] | None = None) -> bytes:
        buffer = io.BytesIO()
        with tarfile.open(fileobj=buffer, mode="w:gz") as tar:
            for name, content in members.items():
                member = tarfile.TarInfo(name)
                member.size = len(content)
                tar.addfile(member, io.BytesIO(content))
            for name, target in (links or {}).items():
                link = tarfile.TarInfo(name)
                link.type, link.linkname = tarfile.SYMTYPE, target
                tar.addfile(link)
        return buffer.getvalue()

    return build


@pytest.fixture
def sleep(mocker: MockerFixture) -> MockType:
    # The module's own clock, so the patch does not reach other threads that call the shared `time.sleep`.
    sleep = mocker.create_autospec(time.sleep)
    mocker.patch.object(download_module, "time", SimpleNamespace(sleep=sleep))
    return sleep
