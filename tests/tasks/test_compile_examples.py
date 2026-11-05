from __future__ import annotations

import argparse
import json
import subprocess
import sys
from typing import TYPE_CHECKING

import pytest

from tasks.compile_examples import block_floor, main, minor_version, resolve

if TYPE_CHECKING:
    from pathlib import Path

_MINOR: int = sys.version_info.minor
_VERSION: str = f"3.{_MINOR}"


def _simplint(folder: Path, rules: list[dict[str, object]]) -> str:
    # A stand-in simplint whose `rules` command prints the given rules, the process boundary the task crosses.
    path = folder / "simplint"
    path.write_text(f"#!{sys.executable}\nimport sys\nsys.stdout.write({json.dumps(rules)!r})\n")
    path.chmod(0o755)
    return str(path)


def _rule(code: str, *groups: dict[str, object]) -> dict[str, object]:
    return {"code": code, "examples": {"parsed": list(groups)}}


def _group(*blocks: tuple[int, str, str], config: str | None = None) -> dict[str, object]:
    return {
        "config": None if config is None else {"source": config},
        "blocks": [
            {"role": role, "file": "example.py", "line": line, "source": source} for line, role, source in blocks
        ],
    }


def test_passes_when_every_block_compiles(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    simplint = _simplint(tmp_path, [_rule("DEMO", _group((3, "ok", "VALUE = 1\n")))])

    assert (main(["--simplint", simplint, "--python", _VERSION]), capsys.readouterr().out) == (0, "")


def test_lists_only_the_blocks_that_fail(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    simplint = _simplint(tmp_path, [_rule("DEMO", _group((3, "ok", "VALUE = 1\n"), (5, "report", "match value:\n")))])

    failures = main(["--simplint", simplint, "--python", _VERSION]), capsys.readouterr().out.splitlines()

    assert (failures[0], [line.startswith(f"DEMO:5:report py3.{_MINOR}: ") for line in failures[1]]) == (1, [True])


def test_ignores_a_failing_block_below_its_floor(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    newer = f'[project]\nrequires-python = ">=3.{_MINOR + 1}"\n'
    simplint = _simplint(tmp_path, [_rule("DEMO", _group((5, "report", "match value:\n"), config=newer))])

    assert (main(["--simplint", simplint, "--python", _VERSION]), capsys.readouterr().out) == (0, "")


def test_warnings_do_not_fail_a_block(tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
    simplint = _simplint(tmp_path, [_rule("DEMO", _group((1, "report", 'PATTERN = "\\d+"\n')))])

    assert (main(["--simplint", simplint, "--python", _VERSION]), capsys.readouterr().out) == (0, "")


@pytest.mark.parametrize(
    ("config", "expected"),
    [
        pytest.param(None, 11, id="no-config"),
        pytest.param('[tool.ruff]\ntarget-version = "py312"\n', 12, id="target-version"),
        pytest.param('[project]\nrequires-python = ">=3.13"\n', 13, id="requires-python"),
        pytest.param('[project]\nrequires-python = ">=3.13"\n[tool.ruff]\ntarget-version = "py311"\n', 11, id="both"),
        pytest.param('# requires-python = ">=3.14"\n', 11, id="commented-out"),
        pytest.param('[project]\nname = "demo"\n', 11, id="neither"),
    ],
)
def test_block_floor_reads_the_group_config(config: str | None, expected: int) -> None:
    assert block_floor(None if config is None else {"source": config}) == expected


@pytest.mark.parametrize("text", ["3", "2.7", "3.x", "3.12.1"])
def test_minor_version_rejects_anything_but_3_x(text: str) -> None:
    with pytest.raises(argparse.ArgumentTypeError, match=r"expected 3\.X"):
        minor_version(text)


def test_main_rejects_a_bad_python_version(tmp_path: Path) -> None:
    with pytest.raises(SystemExit):
        main(["--simplint", _simplint(tmp_path, []), "--python", "3"])


def test_resolve_finds_an_interpreter_of_the_version() -> None:
    interpreter = resolve(_MINOR)
    reported = subprocess.run(
        [interpreter.path, "-c", "import sys; print(sys.version_info.minor)"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout

    assert (interpreter.minor, int(reported)) == (_MINOR, _MINOR)


def test_resolve_rejects_a_version_uv_cannot_find() -> None:
    with pytest.raises(ValueError, match=r"no CPython 3\.99"):
        resolve(99)
