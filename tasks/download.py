"""Download files pinned by address and sha256, and check each against its digest before it is used."""

from __future__ import annotations

import hashlib
import http.client
import shutil
import sys
import time
import urllib.request
from dataclasses import dataclass
from typing import TYPE_CHECKING, TypeVar

if TYPE_CHECKING:
    from collections.abc import Callable
    from pathlib import Path

_T = TypeVar("_T")


@dataclass(frozen=True, slots=True, kw_only=True)
class Pinned:
    url: str
    sha256: str

    @property
    def filename(self) -> str:
        return self.url.rsplit("/", 1)[-1]


def download(pinned: Pinned, folder: Path) -> Path:
    archive = folder / pinned.filename
    if archive.is_file() and _sha256(archive) == pinned.sha256:
        return archive
    folder.mkdir(parents=True, exist_ok=True)
    # Written aside and moved in place once verified, so an interrupted or tampered download never looks cached.
    partial = archive.with_name(f"{archive.name}.part")
    retried(lambda: _save(pinned.url, partial), pinned.url)
    if (digest := _sha256(partial)) != pinned.sha256:
        partial.unlink()
        msg = f"{pinned.url}: sha256 {digest}, pinned {pinned.sha256}"
        raise ValueError(msg)
    partial.replace(archive)
    return archive


# Hashed here rather than by sha256sum, which escapes its output when a Windows path holds a backslash.
def _sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


# Retry what the network or a server can fail transiently, backing off, and let the last failure through.
def retried(action: Callable[[], _T], what: str) -> _T:
    for attempt in range(1, 4):
        try:
            return action()
        except (OSError, http.client.HTTPException) as error:
            print(f"{what}: {error}, retrying", file=sys.stderr, flush=True)
            time.sleep(2**attempt)
    return action()


def _save(url: str, path: Path) -> None:
    with urllib.request.urlopen(url, timeout=60) as response, path.open("wb") as stream:
        shutil.copyfileobj(response, stream)


__all__ = [
    "Pinned",
    "download",
    "retried",
]
