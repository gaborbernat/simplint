from __future__ import annotations

import hashlib
import http.client
import io
import json
import subprocess
import tarfile
import time
import urllib.error
from pathlib import Path
from types import SimpleNamespace
from typing import TYPE_CHECKING, Final

import pytest

from tasks import pgo
from tasks.pgo import Sdist, download, extract, llvm_profdata, main, pin_sdist, read_manifest, write_manifest

if TYPE_CHECKING:
    from pytest_mock import MockerFixture, MockType

_FILES: Final[str] = "https://files.pythonhosted.org/packages/ab/cd"


def test_sdist_is_named_by_its_url() -> None:
    assert Sdist(url=f"{_FILES}/PyDemo-1.0.tar.gz", sha256="").filename == "PyDemo-1.0.tar.gz"


def test_pin_sdist_takes_the_tar_gz_sdist(mocker: MockerFixture) -> None:
    release = _release(
        ("demo-1.0-py3-none-any.whl", "bdist_wheel"), ("demo-1.0.zip", "sdist"), ("demo-1.0.tar.gz", "sdist")
    )
    urlopen = mocker.patch("urllib.request.urlopen", autospec=True, return_value=release)

    assert pin_sdist("demo") == Sdist(url=f"{_FILES}/demo-1.0.tar.gz", sha256="demo-1.0.tar.gz-digest")
    assert urlopen.call_args.args == ("https://pypi.org/pypi/demo/json",)


def _release(*files: tuple[str, str]) -> io.BytesIO:
    urls = [
        {"filename": name, "packagetype": kind, "url": f"{_FILES}/{name}", "digests": {"sha256": f"{name}-digest"}}
        for name, kind in files
    ]
    return io.BytesIO(json.dumps({"urls": urls}).encode())


def test_pin_sdist_rejects_a_wheel_only_release(mocker: MockerFixture) -> None:
    mocker.patch("urllib.request.urlopen", autospec=True, return_value=_release(("demo.whl", "bdist_wheel")))

    with pytest.raises(ValueError, match=r"^the latest release of demo ships no \.tar\.gz sdist$"):
        pin_sdist("demo")


@pytest.fixture
def sleep(mocker: MockerFixture) -> MockType:
    # The module's own clock, so the patch does not reach other threads that call the shared `time.sleep`.
    sleep = mocker.create_autospec(time.sleep)
    mocker.patch.object(pgo, "time", SimpleNamespace(sleep=sleep, monotonic=time.monotonic))
    return sleep


@pytest.mark.usefixtures("sleep")
def test_pin_sdist_retries_a_transient_failure(mocker: MockerFixture, capsys: pytest.CaptureFixture[str]) -> None:
    release = _release(("demo-1.0.tar.gz", "sdist"))
    mocker.patch("urllib.request.urlopen", autospec=True, side_effect=[urllib.error.URLError("reset"), release])

    assert pin_sdist("demo") == Sdist(url=f"{_FILES}/demo-1.0.tar.gz", sha256="demo-1.0.tar.gz-digest")
    assert capsys.readouterr().err == "https://pypi.org/pypi/demo/json: <urlopen error reset>, retrying\n"


def test_main_pins_the_latest_sdists(tmp_path: Path, mocker: MockerFixture) -> None:
    mocker.patch("tasks.pgo.PROJECTS", ("b", "A"))
    mocker.patch(
        "urllib.request.urlopen",
        autospec=True,
        side_effect=lambda url, **_: _release((f"{url.split('/')[-2]}-1.0.tar.gz", "sdist")),
    )
    manifest = tmp_path / "pgo_corpus.toml"

    main(["manifest", "--manifest", str(manifest)])

    assert read_manifest(manifest) == [
        Sdist(url=f"{_FILES}/b-1.0.tar.gz", sha256="b-1.0.tar.gz-digest"),
        Sdist(url=f"{_FILES}/A-1.0.tar.gz", sha256="A-1.0.tar.gz-digest"),
    ]


def test_download_fetches_and_verifies(tmp_path: Path, mocker: MockerFixture) -> None:
    mocker.patch("urllib.request.urlopen", autospec=True, return_value=io.BytesIO(b"archive"))
    sdist = Sdist(url=f"{_FILES}/demo-1.0.tar.gz", sha256=hashlib.sha256(b"archive").hexdigest())

    archive = download(sdist, tmp_path / "cache")

    assert (archive.read_bytes(), [path.name for path in archive.parent.iterdir()]) == (b"archive", ["demo-1.0.tar.gz"])


def test_download_replaces_a_stale_file(tmp_path: Path, mocker: MockerFixture) -> None:
    (tmp_path / "demo-1.0.tar.gz").write_bytes(b"stale")
    mocker.patch("urllib.request.urlopen", autospec=True, return_value=io.BytesIO(b"archive"))

    download(Sdist(url=f"{_FILES}/demo-1.0.tar.gz", sha256=hashlib.sha256(b"archive").hexdigest()), tmp_path)

    assert (tmp_path / "demo-1.0.tar.gz").read_bytes() == b"archive"


def test_download_reuses_a_verified_file(tmp_path: Path, mocker: MockerFixture) -> None:
    urlopen = mocker.patch("urllib.request.urlopen", autospec=True)
    (tmp_path / "demo-1.0.tar.gz").write_bytes(b"cached")

    download(Sdist(url=f"{_FILES}/demo-1.0.tar.gz", sha256=hashlib.sha256(b"cached").hexdigest()), tmp_path)

    urlopen.assert_not_called()


@pytest.mark.usefixtures("sleep")
def test_download_retries_a_truncated_transfer(tmp_path: Path, mocker: MockerFixture) -> None:
    responses = [http.client.IncompleteRead(b"arch"), io.BytesIO(b"archive")]
    mocker.patch("urllib.request.urlopen", autospec=True, side_effect=responses)

    archive = download(Sdist(url=f"{_FILES}/demo.tar.gz", sha256=hashlib.sha256(b"archive").hexdigest()), tmp_path)

    assert archive.read_bytes() == b"archive"


def test_download_gives_up_after_the_last_attempt(tmp_path: Path, mocker: MockerFixture, sleep: MockType) -> None:
    error = urllib.error.HTTPError(_FILES, 503, "Unavailable", None, None)  # type: ignore[arg-type] # no headers
    mocker.patch("urllib.request.urlopen", autospec=True, side_effect=error)

    with pytest.raises(urllib.error.HTTPError, match="Unavailable"):
        download(Sdist(url=f"{_FILES}/demo-1.0.tar.gz", sha256="0" * 64), tmp_path)
    assert [call.args for call in sleep.call_args_list] == [(2,), (4,), (8,)]


def test_download_rejects_a_digest_mismatch(tmp_path: Path, mocker: MockerFixture) -> None:
    mocker.patch("urllib.request.urlopen", autospec=True, return_value=io.BytesIO(b"tampered"))

    with pytest.raises(ValueError, match=r"demo-1\.0\.tar\.gz: sha256 [0-9a-f]{64}, pinned 0{64}"):
        download(Sdist(url=f"{_FILES}/demo-1.0.tar.gz", sha256="0" * 64), tmp_path)
    assert list(tmp_path.iterdir()) == []


def test_extract_keeps_only_python_files(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    archive = tmp_path / "demo-1.0.tar.gz"
    archive.write_bytes(_tar_gz("demo-1.0/demo/__init__.py", "demo-1.0/README.md"))

    extract(archive, tmp_path / "corpus")

    assert [path.name for path in (tmp_path / "corpus").rglob("*") if path.is_file()] == ["__init__.py"]
    assert not capsys.readouterr().out


# An sdist holding the given empty files and a symlink, which is no regular file to extract.
def _tar_gz(*names: str) -> bytes:
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w:gz") as tar:
        for name in names:
            tar.addfile(tarfile.TarInfo(name), io.BytesIO())
        link = tarfile.TarInfo("demo-1.0/link.py")
        link.type, link.linkname = tarfile.SYMTYPE, "demo/__init__.py"
        tar.addfile(link)
    return buffer.getvalue()


@pytest.mark.parametrize(
    ("host", "tool"),
    [
        pytest.param("x86_64-unknown-linux-gnu", "llvm-profdata", id="unix"),
        pytest.param("x86_64-pc-windows-msvc", "llvm-profdata.exe", id="windows"),
    ],
)
def test_llvm_profdata_needs_the_component(tmp_path: Path, mocker: MockerFixture, host: str, tool: str) -> None:
    outputs = {"--version": f"rustc 1.99.1\nhost: {host}\n", "--print": f"{tmp_path}\n"}
    mocker.patch(
        "subprocess.run",
        autospec=True,
        side_effect=lambda command, **_: subprocess.CompletedProcess(command, 0, stdout=outputs[command[1]]),
    )

    with pytest.raises(FileNotFoundError, match=rf"{tool}, run: rustup component add llvm-tools-preview$"):
        llvm_profdata()


def test_train_records_a_profile(tmp_path: Path, mocker: MockerFixture, capsys: pytest.CaptureFixture[str]) -> None:
    calls = _train(tmp_path, mocker, status=1)

    work = tmp_path / "work"
    corpus = work / "corpus"
    check = ["simplint", "check", "--no-cache", "--config"]
    assert [command[:4] for command in calls[2:4]] == [check, check]
    assert [command[5:] for command in calls[2:4]] == [
        [str(corpus)],
        ["--fix", "--unsafe-fixes", "--diff", str(corpus)],
    ]
    assert calls[4][1:] == ["merge", "--output", str(work / "simplint.profdata"), str(work / "profiles")]
    assert sorted(path.relative_to(corpus).as_posix() for path in corpus.rglob("*") if path.is_file()) == [
        "demo-1.0/demo-1.0/demo/__init__.py",
        "demo-1.0/demo-1.0/demo/types.pyi",
    ]
    output = capsys.readouterr().out
    assert "::warning::demo-1.0.tar.gz: skipped 1 Python files, first ../escape.py (" in output
    assert output.endswith(f"profile written to {work / 'simplint.profdata'}, 0.0 MiB\n")


# Run the training over one sdist with every tool simulated, and return the commands; simplint exits with the given
# status.
def _train(tmp_path: Path, mocker: MockerFixture, status: int) -> list[list[str]]:
    payload = _tar_gz("demo-1.0/demo/__init__.py", "demo-1.0/demo/types.pyi", "demo-1.0/README.md", "../escape.py")
    manifest = tmp_path / "pgo_corpus.toml"
    write_manifest(manifest, [Sdist(url=f"{_FILES}/demo-1.0.tar.gz", sha256=hashlib.sha256(payload).hexdigest())])
    mocker.patch("urllib.request.urlopen", autospec=True, return_value=io.BytesIO(payload))
    tool = tmp_path / "sysroot" / "lib" / "rustlib" / "x86_64-unknown-linux-gnu" / "bin" / "llvm-profdata"
    tool.parent.mkdir(parents=True)
    tool.touch()
    outputs = {"--version": "rustc 1.99.1\nhost: x86_64-unknown-linux-gnu\n", "--print": f"{tool.parents[4]}\n"}
    calls: list[list[str]] = []

    def run(command: list[str], **kwargs: Path | bool | int | dict[str, str]) -> subprocess.CompletedProcess[str]:
        del kwargs
        calls.append(command)
        if command[0] == "rustc":
            return subprocess.CompletedProcess(command, 0, stdout=outputs[command[1]])
        if command[0] == str(tool):
            (tmp_path / "work" / "simplint.profdata").write_bytes(b"profile")
            return subprocess.CompletedProcess(command, 0)
        return subprocess.CompletedProcess(command, status)

    mocker.patch("subprocess.run", autospec=True, side_effect=run)
    main(["train", "simplint", str(tmp_path / "work"), "--manifest", str(manifest)])
    return calls


def test_train_stops_when_simplint_fails(tmp_path: Path, mocker: MockerFixture) -> None:
    with pytest.raises(subprocess.CalledProcessError, match="exit status 2"):
        _train(tmp_path, mocker, status=2)


_COMPILING: Final[str] = "   Compiling simplint v0.1.0 (/src)\n"
_SIMPLINT_FN: Final[str] = "_RNvCs1ORA0Rd0xIe_8simplint4main"
_COLD_FN: Final[str] = "_RINvNtCsa1_6fs_err4file4openReECs1ORA0Rd0xIe_8simplint"
_MISSING: Final[str] = "warning: simplint.1-cgu.0: no profile data available for function"


@pytest.fixture
def profile_names(mocker: MockerFixture) -> list[str]:
    names: list[str] = []
    mocker.patch("tasks.pgo.llvm_profdata", autospec=True, return_value=Path("llvm-profdata"))
    mocker.patch(
        "subprocess.run",
        autospec=True,
        side_effect=lambda command, **_: subprocess.CompletedProcess(command, 0, stdout="\n".join(names)),
    )
    return names


@pytest.mark.parametrize(
    ("log", "summary"),
    [
        pytest.param(_COMPILING, "none", id="matching"),
        pytest.param(
            f"{_COMPILING}{_MISSING} {_COLD_FN} Hash = 1\n{_MISSING} rust_begin_unwind Hash = 2\n",
            "{'fs_err': 1, 'rust_begin_unwind': 1}",
            id="cold-crates",
        ),
        pytest.param(f"{_COMPILING}{_MISSING} {_SIMPLINT_FN} Hash = 1\n", "{'simplint': 1}", id="cold-instance"),
    ],
)
def test_verify_passes_a_matching_profile(
    tmp_path: Path, capsys: pytest.CaptureFixture[str], profile_names: list[str], log: str, summary: str
) -> None:
    profile_names.append("_RNvCs1ORA0Rd0xIe_8simplint3run")
    (tmp_path / "build.log").write_text(log)

    main(["verify", str(tmp_path / "build.log"), str(tmp_path / "simplint.profdata")])

    assert capsys.readouterr().out == f"functions without PGO profile data: {summary}\n"


@pytest.mark.parametrize(
    "warning",
    [
        pytest.param(f"{_MISSING} {_SIMPLINT_FN}", id="missing"),
        pytest.param(
            f"warning: simplint.1-cgu.0: function control flow change detected (hash mismatch) {_SIMPLINT_FN} Hash = 3",
            id="mismatch",
        ),
        pytest.param(
            "\x1b[1m\x1b[33mwarning\x1b[0m\x1b[1m:\x1b[0m simplint.1-cgu.0: no profile data available for function"
            f" {_SIMPLINT_FN}",
            id="colored",
        ),
    ],
)
def test_verify_rejects_a_profile_of_another_build(tmp_path: Path, profile_names: list[str], warning: str) -> None:
    profile_names.append("_RNvCs9OTHER0Rd0xI_8simplint3run")
    (tmp_path / "build.log").write_text(f"{_COMPILING}{warning}\n")

    with pytest.raises(SystemExit, match=r"does not match this build, no profile data for functions of simplint$"):
        main(["verify", str(tmp_path / "build.log"), str(tmp_path / "simplint.profdata")])


def test_verify_rejects_a_changed_function_even_of_a_matching_crate(tmp_path: Path, profile_names: list[str]) -> None:
    profile_names.append(_SIMPLINT_FN)
    changed = f"warning: simplint.1-cgu.0: function control flow change detected (hash mismatch) {_SIMPLINT_FN} H = 3"
    (tmp_path / "build.log").write_text(f"{_COMPILING}{changed}\n")

    with pytest.raises(SystemExit, match=r"does not match this build, no profile data for functions of simplint$"):
        main(["verify", str(tmp_path / "build.log"), str(tmp_path / "simplint.profdata")])


def test_verify_rejects_a_log_without_the_build(tmp_path: Path) -> None:
    (tmp_path / "build.log").write_text("error: could not compile\n")

    with pytest.raises(SystemExit, match=r"shows no build of simplint"):
        main(["verify", str(tmp_path / "build.log"), str(tmp_path / "simplint.profdata")])
