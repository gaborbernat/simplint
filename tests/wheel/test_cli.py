from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

import pytest


@pytest.fixture(scope="session")
def simplint() -> str:
    found = shutil.which("simplint", path=str(Path(sys.executable).parent))
    assert found is not None
    return found


def test_check_reports_and_fails(simplint: str, tmp_path: Path) -> None:
    (tmp_path / "module.py").write_text("x = 1\n")

    result = subprocess.run(
        [simplint, "check", "--isolated", "--select", "SMP010", "--output-format", "concise", "module.py"],
        cwd=tmp_path,
        capture_output=True,
        text=True,
        check=False,
    )

    assert (result.returncode, result.stdout.splitlines()) == (
        1,
        [
            "module.py:1:1: SMP010 Single-letter name `x`; only loop targets and comprehensions may use one",
            "Found 1 error.",
        ],
    )


def test_check_fix_rewrites_the_file(simplint: str, tmp_path: Path) -> None:
    module = tmp_path / "module.py"
    module.write_text("# --- helpers ---\nVALUE = 1\n")

    subprocess.run(
        [simplint, "check", "--isolated", "--fix", "--select", "SMP001", "module.py"], cwd=tmp_path, check=True
    )

    assert module.read_text() == "VALUE = 1\n"


def test_rule_prints_its_documentation(simplint: str) -> None:
    result = subprocess.run([simplint, "rule", "SMP114"], capture_output=True, text=True, check=True)

    assert result.stdout.splitlines()[0] == "# dataclass-kw-only (SMP114)"
