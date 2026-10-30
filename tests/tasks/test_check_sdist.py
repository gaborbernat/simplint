from __future__ import annotations

import io
import sys
import tarfile
from pathlib import Path
from typing import TYPE_CHECKING

from tasks.check_sdist import main

if TYPE_CHECKING:
    from pytest_mock import MockerFixture


def test_main_builds_installs_and_tests_the_sdist(mocker: MockerFixture) -> None:
    calls = mocker.patch("subprocess.check_call", autospec=True, side_effect=_build)

    main()

    assert [call.args[0][:3] for call in calls.call_args_list] == [
        ["uv", "build", "--sdist"],
        ["uv", "build", "--wheel"],
        ["uv", "pip", "install"],
        [str(Path(sys.executable).parent / "simplint"), "--version"],
        ["cargo", "test", "--locked"],
    ]


def _build(command: list[str], cwd: Path | None = None) -> None:
    if "--sdist" in command:
        out = Path(command[command.index("--out-dir") + 1])
        out.mkdir(parents=True, exist_ok=True)
        with tarfile.open(out / "simplint-1.0.0.tar.gz", "w:gz") as tar:
            info = tarfile.TarInfo("simplint-1.0.0/Cargo.toml")
            tar.addfile(info, io.BytesIO(b""))
    elif "--wheel" in command:
        out = Path(command[command.index("--out-dir") + 1])
        out.mkdir(parents=True, exist_ok=True)
        (out / "simplint-1.0.0-py3-none-any.whl").touch()
    del cwd
