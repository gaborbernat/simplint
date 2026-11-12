from __future__ import annotations

import subprocess
import sys
from pathlib import Path
from tarfile import open as tar_open
from tempfile import TemporaryDirectory
from typing import Final

#: What the build and the test suite read.
_SHIPPED: Final[tuple[str, ...]] = ("src", "tests", "docs", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml")


def main() -> None:
    # Check that the sdist builds a wheel that runs offline and ships everything its test suite needs.
    #
    # :raises subprocess.CalledProcessError: if a build, install or test compile step fails
    # :raises SystemExit: if the sdist misses a tracked file the build or the tests read
    root = Path(__file__).resolve().parents[1]
    with TemporaryDirectory() as folder:
        at = Path(folder)
        subprocess.check_call(["uv", "build", "--sdist", "--out-dir", str(at / "sdist"), str(root)])
        sdist = next((at / "sdist").glob("*.tar.gz"))
        _ships_every_tracked_file(sdist, root)
        subprocess.check_call(["uv", "build", "--wheel", "--out-dir", str(at / "wheel"), str(sdist)])
        _runs_on_its_own(next((at / "wheel").glob("*.whl")))
        _ships_a_suite_that_compiles(sdist, at / "unpacked")


def _ships_every_tracked_file(sdist: Path, root: Path) -> None:
    tracked = subprocess.run(
        ["git", "ls-files", "--", *_SHIPPED], cwd=root, capture_output=True, text=True, check=True
    ).stdout.splitlines()
    with tar_open(sdist) as tar:
        shipped = {name.partition("/")[2] for name in tar.getnames()}
    if missing := sorted(set(tracked) - shipped):
        msg = f"{sdist.name} misses {', '.join(missing)}"
        raise SystemExit(msg)
    print(f"{sdist.name} ships every tracked file the build and the tests read", flush=True)


def _runs_on_its_own(wheel: Path) -> None:
    # --no-index leaves nothing to fall back on, so a wheel missing its binary cannot install or run
    subprocess.check_call(["uv", "pip", "install", "--python", sys.executable, "--no-index", str(wheel)])
    subprocess.check_call([str(Path(sys.executable).parent / "simplint"), "--version"])
    print(f"{wheel.name} built from the sdist runs with no index behind it", flush=True)


def _ships_a_suite_that_compiles(sdist: Path, at: Path) -> None:
    # what a packager builds before running the suite, which the main test job runs against the same files
    at.mkdir()
    with tar_open(sdist) as tar:
        tar.extractall(at, filter="data")
    subprocess.check_call(["cargo", "test", "--locked", "--no-run"], cwd=next(at.iterdir()))
    print(f"{sdist.name} ships a suite that compiles", flush=True)


if __name__ == "__main__":
    main()


__all__ = [
    "main",
]
