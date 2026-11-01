from __future__ import annotations

import hashlib
import io
import json
import subprocess
import tarfile
from typing import TYPE_CHECKING, Protocol

import pytest

from tasks.ecosystem import PACKAGES, Package, check, download, main

if TYPE_CHECKING:
    from pathlib import Path

    from pytest_mock import MockerFixture


class _Behavior(Protocol):
    def __call__(self, command: list[str], cwd: Path) -> tuple[int, str, str]: ...


class _Run(Protocol):
    def __call__(
        self,
        command: list[str],
        *,
        cwd: Path,
        capture_output: bool,
        text: bool,
        check: bool,
    ) -> subprocess.CompletedProcess[str]: ...


def test_packages_are_pinned_to_pypi_source_archives() -> None:
    assert (len(PACKAGES), PACKAGES[0].url) == (
        25,
        "https://files.pythonhosted.org/packages/source/b/boto3/boto3-1.43.122.tar.gz",
    )


def test_download_fetches_and_verifies(tmp_path: Path, mocker: MockerFixture) -> None:
    payload = b"archive"
    urlopen = mocker.patch("urllib.request.urlopen", autospec=True, return_value=io.BytesIO(payload))
    package = Package(name="demo-pkg", version="1.0", sha256=hashlib.sha256(payload).hexdigest())

    archive = download(package, tmp_path / "cache")

    assert (archive.read_bytes(), archive.name, urlopen.call_args.args) == (
        payload,
        "demo_pkg-1.0.tar.gz",
        ("https://files.pythonhosted.org/packages/source/d/demo-pkg/demo_pkg-1.0.tar.gz",),
    )


def test_download_reuses_the_cache(tmp_path: Path, mocker: MockerFixture) -> None:
    urlopen = mocker.patch("urllib.request.urlopen", autospec=True)
    (tmp_path / "demo-1.0.tar.gz").write_bytes(b"cached")

    download(Package(name="demo", version="1.0", sha256=hashlib.sha256(b"cached").hexdigest()), tmp_path)

    urlopen.assert_not_called()


def test_download_rejects_a_hash_mismatch(tmp_path: Path) -> None:
    archive = tmp_path / "demo-1.0.tar.gz"
    archive.write_bytes(b"tampered")

    with pytest.raises(ValueError, match="does not match its pinned sha256"):
        download(Package(name="demo", version="1.0", sha256="0" * 64), tmp_path)
    assert not archive.exists()


def _clean(command: list[str], cwd: Path) -> tuple[int, str, str]:
    del command, cwd
    return 0, "", ""


def _fails(stderr: str) -> _Behavior:
    def behavior(command: list[str], cwd: Path) -> tuple[int, str, str]:
        del cwd
        return (0, "", "") if "--fix-only" in command else (2, "", stderr)

    return behavior


def _cannot_read(command: list[str], cwd: Path) -> tuple[int, str, str]:
    del cwd
    if command[0] == "ruff" or {"--fix-only", "--diff"} & set(command):
        return 0, "", ""
    return 1, "mod.py:1:1: E902 Permission denied\nFound 1 error.\n", ""


def _cannot_read_like_ruff(command: list[str], cwd: Path) -> tuple[int, str, str]:
    del cwd
    return (0, "", "") if {"--fix-only", "--diff"} & set(command) else (1, "mod.py:1:1: io-error: unreadable\n", "")


# The fix leaves `source` in the module; the simulated Ruff reacts to the markers in it.
def _writes(source: str) -> _Behavior:
    def behavior(command: list[str], cwd: Path) -> tuple[int, str, str]:
        if "--fix-only" in command:
            (cwd / "mod.py").write_text(source)
        return 0, "", ""

    return behavior


def _keeps_fixing(command: list[str], cwd: Path) -> tuple[int, str, str]:
    del cwd
    return (1, "--- mod.py\n+++ mod.py\n", "") if "--diff" in command else (0, "", "")


def _keeps_fixing_where_format_strands_a_pragma(command: list[str], cwd: Path) -> tuple[int, str, str]:
    if command[:2] == ["ruff", "format"]:
        (cwd / "mod.py").write_text("VALUE = (\n    1\n)  # noqa: F401\n")
    return (1, "--- mod.py\n+++ mod.py\n", "") if "--diff" in command else (0, "", "")


@pytest.mark.parametrize(
    ("behavior", "expected"),
    [
        pytest.param(_clean, [], id="clean"),
        pytest.param(_writes("VALUE = 'long'\n"), ["mod.py: new E501"], id="ruff-violation"),
        pytest.param(_writes("VALUE = 'comma'\n"), [], id="formatter-conflict"),
        pytest.param(_writes("VALUE = 'unformatted'\n"), ["ruff format left the code unformatted"], id="formatting"),
        pytest.param(
            _writes("VALUE = 'unsettled'\n"),
            ["a second round of ruff check --fix changed the code"],
            id="ruff-unsettled",
        ),
        pytest.param(_fails("simplint failed\n  Cause: boom\n"), ["check: simplint failed"], id="error"),
        pytest.param(_fails("panic\n"), ["check: panic"], id="error-without-message"),
        pytest.param(_cannot_read, ["check: mod.py:1:1: E902 Permission denied"], id="io-error"),
        pytest.param(_cannot_read_like_ruff, [], id="io-error-ruff-shares"),
        pytest.param(_writes("def broken(:\n"), ["mod.py no longer parses"], id="syntax"),
        pytest.param(_keeps_fixing, ["a second round of simplint check --fix changed the code"], id="not-idempotent"),
        pytest.param(_keeps_fixing_where_format_strands_a_pragma, [], id="stranded-pragma"),
        pytest.param(_writes("VALUE = missing\n"), ["mod.py: new F821"], id="undefined-name"),
    ],
)
def test_check_reports_problems(
    tmp_path: Path, mocker: MockerFixture, behavior: _Behavior, expected: list[str]
) -> None:
    mocker.patch("subprocess.run", autospec=True, side_effect=_tools(behavior))

    assert check(Package(name="demo", version="1.0", sha256=""), _sdist(tmp_path), "simplint") == expected


def test_main_rejects_unpinned_names(tmp_path: Path) -> None:
    with pytest.raises(ValueError, match="not pinned: nope"):
        main(["--cache", str(tmp_path), "nope"])


@pytest.mark.parametrize(
    ("behavior", "expected"),
    [
        pytest.param(_clean, (0, "demo 1.0: ok\n"), id="passes"),
        pytest.param(
            _keeps_fixing, (1, "demo 1.0: a second round of simplint check --fix changed the code\n"), id="fails"
        ),
    ],
)
def test_main_checks_the_selected_packages(
    tmp_path: Path,
    mocker: MockerFixture,
    capsys: pytest.CaptureFixture[str],
    behavior: _Behavior,
    expected: tuple[int, str],
) -> None:
    sdist = _sdist(tmp_path)
    selected = Package(name="demo", version="1.0", sha256=hashlib.sha256(sdist.read_bytes()).hexdigest())
    mocker.patch("tasks.ecosystem.PACKAGES", (selected, Package(name="other", version="2.0", sha256="")))
    mocker.patch("subprocess.run", autospec=True, side_effect=_tools(behavior))

    assert (main(["--cache", str(tmp_path), "demo"]), capsys.readouterr().out) == expected


def _sdist(at: Path) -> Path:
    archive = at / "demo-1.0.tar.gz"
    content = b"VALUE = 1\n"
    with tarfile.open(archive, "w:gz") as tar:
        info = tarfile.TarInfo("demo-1.0/mod.py")
        info.size = len(content)
        tar.addfile(info, io.BytesIO(content))
    return archive


def _tools(behavior: _Behavior) -> _Run:
    conflict_warning = (
        "warning: The following rule may cause conflicts when used with the formatter: "
        "`missing-trailing-comma` (`COM812`).\n"
    )

    def run(
        command: list[str],
        *,
        cwd: Path,
        capture_output: bool,
        text: bool,
        check: bool,
    ) -> subprocess.CompletedProcess[str]:
        del capture_output, text, check
        if command[:2] == ["ruff", "check"] and "json" in command:
            markers = {"missing": "F821"} if "--ignore-noqa" in command else {"long": "E501", "comma": "COM812"}
            ignored = command[command.index("--ignore") + 1].split(",") if "--ignore" in command else []
            stdout = json.dumps([
                {"filename": str(path.resolve()), "code": rule}
                for path in cwd.rglob("*.py")
                for marker, rule in markers.items()
                if marker in path.read_text() and rule not in ignored
            ])
            return subprocess.CompletedProcess(command, 0, stdout, "")
        if command[:2] == ["ruff", "check"] and "concise" in command:
            code, stdout, stderr = behavior(command, cwd)
            return subprocess.CompletedProcess(command, code, stdout, stderr)
        if command[:2] == ["ruff", "check"] and "--diff" in command:
            unsettled = any("unsettled" in path.read_text() for path in cwd.rglob("*.py"))
            return subprocess.CompletedProcess(command, int(unsettled), "", "")
        if command[:2] == ["ruff", "format"] and "--check" in command:
            unformatted = any("unformatted" in path.read_text() for path in cwd.rglob("*.py"))
            return subprocess.CompletedProcess(command, int(unformatted), "", conflict_warning)
        if command[:2] == ["ruff", "format"]:
            behavior(command, cwd)
        if command[0] == "ruff":
            return subprocess.CompletedProcess(command, 0, "", "")
        code, stdout, stderr = behavior(command, cwd)
        return subprocess.CompletedProcess(command, code, stdout, stderr)

    return run
