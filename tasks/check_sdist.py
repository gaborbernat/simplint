from __future__ import annotations

import subprocess
import sys
from pathlib import Path
from tarfile import open as tar_open
from tempfile import TemporaryDirectory


def main() -> None:
    """
    Check that the sdist builds a wheel that runs offline and ships a test suite that passes.

    :raises subprocess.CalledProcessError: if a build, install or test step fails
    """
    with TemporaryDirectory() as folder:
        at = Path(folder)
        sdist = _build_sdist(at / "sdist")
        _runs_on_its_own(_wheel_from(sdist, at / "wheel"))
        _ships_a_suite_that_passes(sdist, at / "unpacked")


def _build_sdist(at: Path) -> Path:
    subprocess.check_call(["uv", "build", "--sdist", "--out-dir", str(at), str(Path(__file__).resolve().parents[1])])
    return next(at.glob("*.tar.gz"))


def _wheel_from(sdist: Path, at: Path) -> Path:
    subprocess.check_call(["uv", "build", "--wheel", "--out-dir", str(at), str(sdist)])
    return next(at.glob("*.whl"))


def _runs_on_its_own(wheel: Path) -> None:
    # --no-index leaves nothing to fall back on, so a wheel missing its binary cannot install or run
    subprocess.check_call(["uv", "pip", "install", "--python", sys.executable, "--no-index", str(wheel)])
    subprocess.check_call([str(Path(sys.executable).parent / "simplint"), "--version"])
    print(f"{wheel.name} built from the sdist runs with no index behind it")


def _ships_a_suite_that_passes(sdist: Path, at: Path) -> None:
    # what a packager runs: unpack the sdist and run the suite it ships
    at.mkdir()
    with tar_open(sdist) as tar:
        tar.extractall(at, filter="data")
    root = next(at.iterdir())
    subprocess.check_call(["cargo", "test", "--locked"], cwd=root)
    print(f"{sdist.name} passes the suite it ships")


if __name__ == "__main__":
    main()


__all__ = [
    "main",
]
