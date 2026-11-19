from __future__ import annotations

import hashlib
import io
import tarfile
from typing import TYPE_CHECKING

from tasks.ecosystem import Package
from tasks.wheel_trial import unpack

if TYPE_CHECKING:
    from pathlib import Path


def test_source_and_executable_can_share_a_package_name(tmp_path: Path) -> None:
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
    assert root.parent != tmp_path / "ruff"
    assert (root / "example.py").read_bytes() == source
