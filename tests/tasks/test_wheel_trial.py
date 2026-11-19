from __future__ import annotations

import hashlib
import io
import os
import tarfile
from typing import TYPE_CHECKING

import pytest

from tasks.ecosystem import Package
from tasks.wheel_trial import unpack

if TYPE_CHECKING:
    from pathlib import Path


@pytest.mark.parametrize(
    "alias",
    [False, pytest.param(True, marks=pytest.mark.skipif(os.name == "nt", reason="POSIX temporary directory alias"))],
)
def test_source_and_executable_can_share_a_package_name(tmp_path: Path, *, alias: bool) -> None:
    if alias:
        link = tmp_path / "alias"
        location = tmp_path / "work"
        location.mkdir()
        link.symlink_to(location, target_is_directory=True)
        tmp_path = link
    (tmp_path / "ruff" / "cli").mkdir(parents=True)
    cache = tmp_path / "archives"
    cache.mkdir()
    archive = cache / "ruff-1.tar.gz"
    with tarfile.open(archive, "w:gz") as dist:
        source = b"value = 1\n"
        member = tarfile.TarInfo("ruff-1/example.py")
        member.size = len(source)
        dist.addfile(member, io.BytesIO(source))
    package = Package(name="ruff", version="1", sha256=hashlib.sha256(archive.read_bytes()).hexdigest())
    root = unpack(package, tmp_path)
    assert root == root.resolve()
    assert root.parent != tmp_path / "ruff"
    assert (root / "example.py").read_bytes() == source
