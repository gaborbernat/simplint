from __future__ import annotations

import hashlib
import io
import json
import subprocess
import urllib.error
from pathlib import Path
from typing import TYPE_CHECKING, Final

import pytest

from tasks.download import Pinned
from tasks.pgo import extract, llvm_profdata, main, pin_sdist, read_manifest, write_manifest

if TYPE_CHECKING:
    from collections.abc import Callable

    from pytest_mock import MockerFixture


def test_pin_sdist_takes_the_tar_gz_sdist(mocker: MockerFixture, files_url: str) -> None:
    release = _release(
        files_url,
        ("demo-1.0-py3-none-any.whl", "bdist_wheel"),
        ("demo-1.0.zip", "sdist"),
        ("demo-1.0.tar.gz", "sdist"),
    )
    urlopen = mocker.patch("urllib.request.urlopen", autospec=True, return_value=release)

    assert pin_sdist("demo") == Pinned(url=f"{files_url}/demo-1.0.tar.gz", sha256="demo-1.0.tar.gz-digest")
    assert urlopen.call_args.args == ("https://pypi.org/pypi/demo/json",)


def _release(files_url: str, *files: tuple[str, str]) -> io.BytesIO:
    urls = [
        {"filename": name, "packagetype": kind, "url": f"{files_url}/{name}", "digests": {"sha256": f"{name}-digest"}}
        for name, kind in files
    ]
    return io.BytesIO(json.dumps({"urls": urls}).encode())


def test_pin_sdist_rejects_a_wheel_only_release(mocker: MockerFixture, files_url: str) -> None:
    mocker.patch("urllib.request.urlopen", autospec=True, return_value=_release(files_url, ("demo.whl", "bdist_wheel")))

    with pytest.raises(ValueError, match=r"^the latest release of demo ships no \.tar\.gz sdist$"):
        pin_sdist("demo")


@pytest.mark.usefixtures("sleep")
def test_pin_sdist_retries_a_transient_failure(
    mocker: MockerFixture, capsys: pytest.CaptureFixture[str], files_url: str
) -> None:
    release = _release(files_url, ("demo-1.0.tar.gz", "sdist"))
    mocker.patch("urllib.request.urlopen", autospec=True, side_effect=[urllib.error.URLError("reset"), release])

    assert pin_sdist("demo") == Pinned(url=f"{files_url}/demo-1.0.tar.gz", sha256="demo-1.0.tar.gz-digest")
    assert capsys.readouterr().err == "https://pypi.org/pypi/demo/json: <urlopen error reset>, retrying\n"


def test_main_pins_the_latest_sdists(tmp_path: Path, mocker: MockerFixture, files_url: str) -> None:
    mocker.patch("tasks.pgo.PROJECTS", ("b", "A"))
    mocker.patch(
        "urllib.request.urlopen",
        autospec=True,
        side_effect=lambda url, **_: _release(files_url, (f"{url.split('/')[-2]}-1.0.tar.gz", "sdist")),
    )
    manifest = tmp_path / "pgo_corpus.toml"

    main(["manifest", "--manifest", str(manifest)])

    assert read_manifest(manifest) == [
        Pinned(url=f"{files_url}/b-1.0.tar.gz", sha256="b-1.0.tar.gz-digest"),
        Pinned(url=f"{files_url}/A-1.0.tar.gz", sha256="A-1.0.tar.gz-digest"),
    ]


def test_extract_keeps_only_python_files(
    tmp_path: Path, capsys: pytest.CaptureFixture[str], tar_gz: Callable[..., bytes]
) -> None:
    archive = tmp_path / "demo-1.0.tar.gz"
    archive.write_bytes(
        tar_gz(
            dict.fromkeys(("demo-1.0/demo/__init__.py", "demo-1.0/README.md"), b""),
            {"demo-1.0/link.py": "demo/__init__.py"},
        )
    )

    extract(archive, tmp_path / "corpus")

    assert [path.name for path in (tmp_path / "corpus").rglob("*") if path.is_file()] == ["__init__.py"]
    assert not capsys.readouterr().out


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


def test_train_records_a_profile(
    tmp_path: Path,
    mocker: MockerFixture,
    capsys: pytest.CaptureFixture[str],
    tar_gz: Callable[..., bytes],
    files_url: str,
) -> None:
    calls = _train(tmp_path, mocker, tar_gz, files_url, status=1)

    work = tmp_path / "work"
    corpus = work / "corpus"
    cached = ["--cache-dir", str(work / "cache"), str(corpus)]
    assert {tuple(command[:3]) for command in calls[2:6]} == {("simplint", "check", "--config")}
    assert [command[4:] for command in calls[2:6]] == [
        ["--no-cache", str(corpus)],
        ["--no-cache", "--fix", "--unsafe-fixes", "--diff", str(corpus)],
        cached,
        cached,
    ]
    assert calls[6][1:] == ["merge", "--output", str(work / "simplint.profdata"), str(work / "profiles")]
    assert sorted(path.relative_to(corpus).as_posix() for path in corpus.rglob("*") if path.is_file()) == [
        "demo-1.0/demo-1.0/demo/__init__.py",
        "demo-1.0/demo-1.0/demo/types.pyi",
    ]
    output = capsys.readouterr().out
    assert "::warning::demo-1.0.tar.gz: skipped 1 Python files, first ../escape.py (" in output
    assert output.endswith(f"profile written to {work / 'simplint.profdata'}, 0.0 MiB\n")


# Run the training over one sdist with every tool simulated, and return the commands; simplint exits with the given
# status.
def _train(
    tmp_path: Path, mocker: MockerFixture, tar_gz: Callable[..., bytes], files_url: str, status: int
) -> list[list[str]]:
    payload = tar_gz(
        dict.fromkeys(
            ("demo-1.0/demo/__init__.py", "demo-1.0/demo/types.pyi", "demo-1.0/README.md", "../escape.py"), b""
        ),
        {"demo-1.0/link.py": "demo/__init__.py"},
    )
    manifest = tmp_path / "pgo_corpus.toml"
    write_manifest(manifest, [Pinned(url=f"{files_url}/demo-1.0.tar.gz", sha256=hashlib.sha256(payload).hexdigest())])
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


def test_train_stops_when_simplint_fails(
    tmp_path: Path, mocker: MockerFixture, tar_gz: Callable[..., bytes], files_url: str
) -> None:
    with pytest.raises(subprocess.CalledProcessError, match="exit status 2"):
        _train(tmp_path, mocker, tar_gz, files_url, status=2)


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
