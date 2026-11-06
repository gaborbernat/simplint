from __future__ import annotations

import email.message
import hashlib
import http.client
import io
import urllib.error
from typing import TYPE_CHECKING, Final

import pytest

from tasks.download import Pinned, download

if TYPE_CHECKING:
    from pathlib import Path

    from pytest_mock import MockerFixture, MockType

_FILES: Final[str] = "https://files.pythonhosted.org/packages/ab/cd"


def test_pinned_is_named_by_its_url() -> None:
    assert Pinned(url=f"{_FILES}/PyDemo-1.0.tar.gz", sha256="").filename == "PyDemo-1.0.tar.gz"


def test_download_fetches_and_verifies(tmp_path: Path, mocker: MockerFixture) -> None:
    mocker.patch("urllib.request.urlopen", autospec=True, return_value=io.BytesIO(b"archive"))
    pinned = Pinned(url=f"{_FILES}/demo-1.0.tar.gz", sha256=hashlib.sha256(b"archive").hexdigest())

    archive = download(pinned, tmp_path / "cache")

    assert (archive.read_bytes(), [path.name for path in archive.parent.iterdir()]) == (b"archive", ["demo-1.0.tar.gz"])


def test_download_replaces_a_stale_file(tmp_path: Path, mocker: MockerFixture) -> None:
    (tmp_path / "demo-1.0.tar.gz").write_bytes(b"stale")
    mocker.patch("urllib.request.urlopen", autospec=True, return_value=io.BytesIO(b"archive"))

    download(Pinned(url=f"{_FILES}/demo-1.0.tar.gz", sha256=hashlib.sha256(b"archive").hexdigest()), tmp_path)

    assert (tmp_path / "demo-1.0.tar.gz").read_bytes() == b"archive"


def test_download_reuses_a_verified_file(tmp_path: Path, mocker: MockerFixture) -> None:
    urlopen = mocker.patch("urllib.request.urlopen", autospec=True)
    (tmp_path / "demo-1.0.tar.gz").write_bytes(b"cached")

    download(Pinned(url=f"{_FILES}/demo-1.0.tar.gz", sha256=hashlib.sha256(b"cached").hexdigest()), tmp_path)

    urlopen.assert_not_called()


@pytest.mark.usefixtures("sleep")
def test_download_retries_a_truncated_transfer(tmp_path: Path, mocker: MockerFixture) -> None:
    responses = [http.client.IncompleteRead(b"arch"), io.BytesIO(b"archive")]
    mocker.patch("urllib.request.urlopen", autospec=True, side_effect=responses)

    archive = download(Pinned(url=f"{_FILES}/demo.tar.gz", sha256=hashlib.sha256(b"archive").hexdigest()), tmp_path)

    assert archive.read_bytes() == b"archive"


def test_download_gives_up_after_the_last_attempt(tmp_path: Path, mocker: MockerFixture, sleep: MockType) -> None:
    error = urllib.error.HTTPError(_FILES, 503, "Unavailable", email.message.Message(), None)
    mocker.patch("urllib.request.urlopen", autospec=True, side_effect=error)

    with pytest.raises(urllib.error.HTTPError, match="Unavailable"):
        download(Pinned(url=f"{_FILES}/demo-1.0.tar.gz", sha256="0" * 64), tmp_path)
    assert [call.args for call in sleep.call_args_list] == [(2,), (4,), (8,)]


def test_download_rejects_a_digest_mismatch(tmp_path: Path, mocker: MockerFixture) -> None:
    mocker.patch("urllib.request.urlopen", autospec=True, return_value=io.BytesIO(b"tampered"))

    with pytest.raises(ValueError, match=r"demo-1\.0\.tar\.gz: sha256 [0-9a-f]{64}, pinned 0{64}"):
        download(Pinned(url=f"{_FILES}/demo-1.0.tar.gz", sha256="0" * 64), tmp_path)
    assert list(tmp_path.iterdir()) == []
