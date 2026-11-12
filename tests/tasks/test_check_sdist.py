from __future__ import annotations

import io
import subprocess
import sys
import tarfile
from pathlib import Path
from typing import TYPE_CHECKING

import pytest

from tasks.check_sdist import main

if TYPE_CHECKING:
    from pytest_mock import MockerFixture


def test_main_builds_installs_and_compiles_the_sdist(mocker: MockerFixture) -> None:
    mocker.patch("subprocess.run", autospec=True, return_value=_tracked("Cargo.toml\n"))
    calls = mocker.patch("subprocess.check_call", autospec=True, side_effect=_build)

    main()

    assert [call.args[0][:4] for call in calls.call_args_list] == [
        ["uv", "build", "--sdist", "--out-dir"],
        ["uv", "build", "--wheel", "--out-dir"],
        ["uv", "pip", "install", "--python"],
        [str(Path(sys.executable).parent / "simplint"), "--version"],
        ["cargo", "test", "--locked", "--no-run"],
    ]


def _tracked(stdout: str) -> subprocess.CompletedProcess[str]:
    return subprocess.CompletedProcess(["git", "ls-files"], 0, stdout=stdout)


def _build(command: list[str], cwd: Path | None = None) -> None:
    if "--sdist" in command:
        out = Path(command[command.index("--out-dir") + 1])
        out.mkdir(parents=True, exist_ok=True)
        with tarfile.open(out / "simplint-1.0.0.tar.gz", "w:gz") as tar:
            tar.addfile(tarfile.TarInfo("simplint-1.0.0/Cargo.toml"), io.BytesIO(b""))
    elif "--wheel" in command:
        out = Path(command[command.index("--out-dir") + 1])
        out.mkdir(parents=True, exist_ok=True)
        (out / "simplint-1.0.0-py3-none-any.whl").touch()
    del cwd


def test_main_fails_when_the_sdist_misses_a_tracked_file(mocker: MockerFixture) -> None:
    mocker.patch("subprocess.run", autospec=True, return_value=_tracked("Cargo.toml\ntests/fixtures/a.py\n"))
    calls = mocker.patch("subprocess.check_call", autospec=True, side_effect=_build)

    with pytest.raises(SystemExit, match=r"^simplint-1\.0\.0\.tar\.gz misses tests/fixtures/a\.py$"):
        main()

    assert len(calls.call_args_list) == 1
