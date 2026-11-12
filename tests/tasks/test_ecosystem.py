from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
import sys
import tomllib
import warnings
from concurrent.futures import ThreadPoolExecutor
from contextlib import nullcontext
from dataclasses import dataclass
from pathlib import Path
from subprocess import run as real_run
from typing import TYPE_CHECKING, Final, Protocol, TypedDict, Unpack

import pytest

from tasks.ecosystem import PACKAGES, Package, check, main
from tasks.interpreters import Interpreter

if TYPE_CHECKING:
    from collections.abc import Callable

    from pytest_mock import MockerFixture, MockType

_CURRENT: Final[Interpreter] = Interpreter(minor=sys.version_info.minor, path=sys.executable)


@dataclass(frozen=True, slots=True, kw_only=True)
class _Outcome:
    code: int = 0
    stdout: str = ""
    stderr: str = ""


class _Behavior(Protocol):
    def __call__(self, command: list[str], cwd: Path) -> _Outcome: ...


class _RunOptions(TypedDict, total=False):
    cwd: Path
    env: dict[str, str]
    capture_output: bool
    text: bool
    check: bool
    input: str


class _Run(Protocol):
    def __call__(self, command: list[str], **options: Unpack[_RunOptions]) -> subprocess.CompletedProcess[str]: ...


class _TarGz(Protocol):
    def __call__(self, members: dict[str, bytes], links: dict[str, str] | None = None) -> bytes: ...


@pytest.fixture
def archive(tmp_path: Path, tar_gz: _TarGz) -> Path:
    path = tmp_path / "demo-1.0.tar.gz"
    path.write_bytes(tar_gz({"demo-1.0/mod.py": b"VALUE = 1\n"}))
    return path


@pytest.fixture
def package(archive: Path) -> Package:
    return Package(name="demo", version="1.0", sha256=hashlib.sha256(archive.read_bytes()).hexdigest())


@pytest.fixture
def run_tools(mocker: MockerFixture) -> Callable[[_Behavior], MockType]:
    def patch(behavior: _Behavior) -> MockType:
        return mocker.patch("subprocess.run", autospec=True, side_effect=_tools(behavior))

    return patch


@pytest.fixture
def pinned(mocker: MockerFixture, package: Package) -> None:
    mocker.patch("tasks.ecosystem.PACKAGES", (package, Package(name="other", version="2.0", sha256="")))


@pytest.fixture
def in_threads(mocker: MockerFixture) -> None:
    # Threads share the fake tools the test patches in, where processes would not.
    mocker.patch("tasks.ecosystem.ProcessPoolExecutor", ThreadPoolExecutor)


@pytest.fixture
def workspace(tmp_path: Path, mocker: MockerFixture) -> Path:
    folder = tmp_path / "workspace"
    folder.mkdir()
    mocker.patch("tasks.ecosystem.TemporaryDirectory", autospec=True, return_value=nullcontext(str(folder)))
    return folder


def test_packages_are_pinned_to_pypi_source_archives() -> None:
    assert (len(PACKAGES), PACKAGES[0].url) == (
        124,
        "https://files.pythonhosted.org/packages/source/b/boto3/boto3-1.43.124.tar.gz",
    )


def test_tox_pins_the_ecosystem_ruff_to_the_crates_version(root: Path) -> None:
    crates = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["dependencies"]["ruff"]
    tox = tomllib.loads((root / "tox.toml").read_text(encoding="utf-8"))
    assert tox["env"]["ecosystem"]["deps"] == [f"ruff=={crates.removeprefix('=')}"]


def test_main_rejects_unpinned_names(tmp_path: Path) -> None:
    with pytest.raises(ValueError, match="not pinned: nope"):
        main(["--cache", str(tmp_path), "nope"])


def _clean(command: list[str], cwd: Path) -> _Outcome:
    del command, cwd
    return _Outcome()


def _fails(stderr: str) -> _Behavior:
    def behavior(command: list[str], cwd: Path) -> _Outcome:
        del cwd
        return _Outcome() if "--fix-only" in command else _Outcome(code=2, stderr=stderr)

    return behavior


def _refuses_unless_ignored(command: list[str], cwd: Path) -> _Outcome:
    del cwd
    if command[0] == "ruff" or command[2:4] == ["--ignore", "SMP301"]:
        return _Outcome()
    conflict = (
        "  - SMP301 (future-annotations) and Ruff's I002 (missing-required-import) undo each other's fixes.\n"
        "    Why: SMP301 removes the import; I002 adds it back.\n"
        "    Fix: drop the import from `required-imports`, or ignore SMP301 in `[tool.simplint]`.\n"
    )
    return _Outcome(code=2, stderr=f"simplint failed\n  Cause: rules work against each other:\n{conflict}")


def _cannot_read(command: list[str], cwd: Path) -> _Outcome:
    del cwd
    if command[0] == "ruff" or {"--fix-only", "--diff"} & set(command):
        return _Outcome()
    return _Outcome(code=1, stdout="mod.py:1:1: E902 Permission denied\nFound 1 error.\n")


def _cannot_read_like_ruff(command: list[str], cwd: Path) -> _Outcome:
    del cwd
    if {"--fix-only", "--diff"} & set(command):
        return _Outcome()
    return _Outcome(code=1, stdout="mod.py:1:1: io-error: unreadable\n")


# The fix leaves `source` in the module; the simulated Ruff reacts to the markers in it.
def _writes(source: str) -> _Behavior:
    def behavior(command: list[str], cwd: Path) -> _Outcome:
        if "--fix-only" in command:
            (cwd / "mod.py").write_text(source)
        return _Outcome()

    return behavior


def _keeps_fixing(command: list[str], cwd: Path) -> _Outcome:
    del cwd
    return _Outcome(code=1, stdout="--- mod.py\n+++ mod.py\n") if "--diff" in command else _Outcome()


# Ruff's own fixes in `mod.py` are withdrawn before simplint runs, in both trees, so a second round of either tool is
# not counted there; simplint's second round still counts in the files Ruff settles.
def _keeps_fixing_where_ruff_never_settles(fixing: str) -> _Behavior:
    def behavior(command: list[str], cwd: Path) -> _Outcome:
        if command[0] == "ruff" and "concise" in command:
            for tree in (cwd, cwd.parent.parent / "original" / cwd.name):
                (tree / "mod.py").write_text("VALUE = 'unsettled'\n")
            return _Outcome()
        return _Outcome(code=1, stdout=f"--- {fixing}\n+++ {fixing}\n") if "--diff" in command else _Outcome()

    return behavior


def _keeps_fixing_where_format_strands_a_pragma(command: list[str], cwd: Path) -> _Outcome:
    if command[:2] == ["ruff", "format"]:
        (cwd / "mod.py").write_text("VALUE = (\n    1\n)  # noqa: F401\n")
    return _keeps_fixing(command, cwd)


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
        pytest.param(_refuses_unless_ignored, [], id="conflicting-rule-ignored"),
        pytest.param(_cannot_read, ["check: mod.py:1:1: E902 Permission denied"], id="io-error"),
        pytest.param(_cannot_read_like_ruff, [], id="io-error-ruff-shares"),
        pytest.param(_writes("def broken(:\n"), ["mod.py no longer parses"], id="syntax"),
        pytest.param(_keeps_fixing, ["a second round of simplint check --fix changed the code"], id="not-idempotent"),
        pytest.param(_keeps_fixing_where_format_strands_a_pragma, [], id="stranded-pragma"),
        pytest.param(_keeps_fixing_where_ruff_never_settles("mod.py"), [], id="ruff-unsettled-baseline"),
        pytest.param(
            _keeps_fixing_where_ruff_never_settles("other.py"),
            ["a second round of simplint check --fix changed the code"],
            id="ruff-unsettled-elsewhere",
        ),
        pytest.param(_writes("VALUE = missing\n"), ["mod.py: new F821"], id="undefined-name"),
    ],
)
def test_check_reports_problems(
    package: Package,
    archive: Path,
    run_tools: Callable[[_Behavior], MockType],
    behavior: _Behavior,
    expected: list[str],
) -> None:
    run_tools(behavior)

    assert check(package, archive, "simplint", None) == expected


@pytest.mark.parametrize(
    ("target", "expected"),
    [
        pytest.param(None, [], id="parser"),
        pytest.param(_CURRENT, ["mod.py no longer parses"], id="compiler"),
    ],
)
def test_check_parses_or_compiles_the_fixed_files(
    package: Package,
    archive: Path,
    run_tools: Callable[[_Behavior], MockType],
    target: Interpreter | None,
    expected: list[str],
) -> None:
    # A `return` outside a function parses but does not compile, so only the target's compiler rejects it.
    run_tools(_writes("return 1\n"))

    assert check(package, archive, "simplint", target) == expected


def test_check_asks_simplint_for_the_target_release(
    package: Package, archive: Path, run_tools: Callable[[_Behavior], MockType]
) -> None:
    run = run_tools(_clean)

    check(package, archive, "simplint", _CURRENT)

    assert ["simplint", "check", "--target-version", f"py3{_CURRENT.minor}"] in [
        call.args[0][:4] for call in run.call_args_list
    ]


@pytest.mark.usefixtures("in_threads", "pinned")
def test_main_accepts_a_target_version(
    tmp_path: Path, run_tools: Callable[[_Behavior], MockType], capsys: pytest.CaptureFixture[str]
) -> None:
    run_tools(_clean)

    status = main(["--cache", str(tmp_path), "--target-version", f"3.{_CURRENT.minor}", "demo"])

    assert (status, capsys.readouterr().out) == (0, "demo 1.0: ok\n")


@pytest.mark.usefixtures("in_threads", "pinned")
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
    run_tools: Callable[[_Behavior], MockType],
    capsys: pytest.CaptureFixture[str],
    behavior: _Behavior,
    expected: tuple[int, str],
) -> None:
    run_tools(behavior)

    assert (main(["--cache", str(tmp_path), "demo"]), capsys.readouterr().out) == expected


@pytest.mark.usefixtures("in_threads")
def test_main_reports_in_the_pinned_order(
    archive: Path,
    mocker: MockerFixture,
    run_tools: Callable[[_Behavior], MockType],
    capsys: pytest.CaptureFixture[str],
) -> None:
    shutil.copy(archive, archive.with_name("second-2.0.tar.gz"))
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    mocker.patch(
        "tasks.ecosystem.PACKAGES",
        (Package(name="second", version="2.0", sha256=digest), Package(name="demo", version="1.0", sha256=digest)),
    )
    run_tools(_clean)

    assert (main(["--cache", str(archive.parent)]), capsys.readouterr().out) == (0, "second 2.0: ok\ndemo 1.0: ok\n")


def test_check_hides_the_user_configuration(
    package: Package,
    archive: Path,
    mocker: MockerFixture,
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
) -> None:
    user_config = tmp_path / "user"
    (user_config / "ruff").mkdir(parents=True)
    (user_config / "ruff" / "ruff.toml").write_text("select = ['ALL']\n")
    monkeypatch.setenv("XDG_CONFIG_HOME", str(user_config))
    fake = _tools(_clean)
    listings: list[list[str]] = []

    def run(command: list[str], **options: Unpack[_RunOptions]) -> subprocess.CompletedProcess[str]:
        listings.append(sorted(path.name for path in Path(options["env"]["XDG_CONFIG_HOME"]).iterdir()))
        return fake(command, **options)

    mocker.patch("subprocess.run", autospec=True, side_effect=run)

    check(package, archive, "simplint", None)

    assert set(map(tuple, listings)) == {()}


def test_check_drops_ruff_environment_overrides(
    package: Package, archive: Path, monkeypatch: pytest.MonkeyPatch, run_tools: Callable[[_Behavior], MockType]
) -> None:
    monkeypatch.setenv("RUFF_LINE_LENGTH", "10")
    run = run_tools(_clean)

    check(package, archive, "simplint", None)

    environments = [call.kwargs["env"] for call in run.call_args_list]
    assert {key for environment in environments for key in environment if key.startswith("RUFF_")} == {"RUFF_CACHE_DIR"}


def test_check_keeps_ruff_cache_in_the_workspace(
    package: Package, archive: Path, run_tools: Callable[[_Behavior], MockType], workspace: Path
) -> None:
    run = run_tools(_clean)

    check(package, archive, "simplint", None)

    caches = {Path(call.kwargs["env"]["RUFF_CACHE_DIR"]) for call in run.call_args_list}
    assert caches == {workspace / "cache"}


def test_check_does_not_warn_about_invalid_escapes(
    package: Package, tmp_path: Path, run_tools: Callable[[_Behavior], MockType], tar_gz: _TarGz
) -> None:
    run_tools(_clean)
    archive = tmp_path / "demo-1.0.tar.gz"
    archive.write_bytes(tar_gz({"demo-1.0/mod.py": b'VALUE = "\\d"\n'}))

    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        problems = check(package, archive, "simplint", None)

    assert (problems, [warning for warning in caught if issubclass(warning.category, SyntaxWarning)]) == ([], [])


def _tools(behavior: _Behavior) -> _Run:
    # The uv and interpreter calls run for real, through the function bound before the tests patch `subprocess.run`;
    # the tools that act on a tree are the fake's.
    def run(command: list[str], **options: Unpack[_RunOptions]) -> subprocess.CompletedProcess[str]:
        if command[0] == "uv" or command[1:2] == ["-I"]:
            return real_run(
                command,
                cwd=options.get("cwd"),
                input=options.get("input"),
                capture_output=True,
                text=True,
                check=options.get("check", False),
            )
        return _fake(behavior, command, options["cwd"])

    return run


# The simulated Ruff answers by the markers the fix left in the files; the runs a test varies go to `behavior`.
def _fake(behavior: _Behavior, command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    if command[0] != "ruff" or (command[:2] == ["ruff", "check"] and "concise" in command):
        outcome = behavior(command, cwd)
        return subprocess.CompletedProcess(command, outcome.code, outcome.stdout, outcome.stderr)
    if command[:2] == ["ruff", "check"] and "json" in command:
        return subprocess.CompletedProcess(command, 0, _ruff_report(command, cwd), "")
    if command[:2] == ["ruff", "check"] and "--diff" in command:
        unsettled = _marked(cwd, "unsettled")
        return subprocess.CompletedProcess(command, int(unsettled), "--- mod.py\n+++ mod.py\n" * unsettled, "")
    if command[:2] == ["ruff", "format"] and "--check" in command:
        conflict_warning = (
            "warning: The following rule may cause conflicts when used with the formatter: "
            "`missing-trailing-comma` (`COM812`).\n"
        )
        return subprocess.CompletedProcess(command, int(_marked(cwd, "unformatted")), "", conflict_warning)
    if command[:2] == ["ruff", "format"]:
        behavior(command, cwd)
    return subprocess.CompletedProcess(command, 0, "", "")


def _ruff_report(command: list[str], cwd: Path) -> str:
    markers = {"missing": "F821"} if "--ignore-noqa" in command else {"long": "E501", "comma": "COM812"}
    ignored = command[command.index("--ignore") + 1].split(",") if "--ignore" in command else []
    return json.dumps([
        {"filename": str(path.resolve()), "code": rule}
        for path in cwd.rglob("*.py")
        for marker, rule in markers.items()
        if marker in path.read_text() and rule not in ignored
    ])


def _marked(cwd: Path, marker: str) -> bool:
    return any(marker in path.read_text() for path in cwd.rglob("*.py"))
