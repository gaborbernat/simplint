from __future__ import annotations

import subprocess
import sys
from pathlib import Path
from tarfile import open as tar_open
from tempfile import TemporaryDirectory


def main() -> None:
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
        ["git", "ls-files", "--", "src", "tests", "docs", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml"],
        cwd=root,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.splitlines()
    with tar_open(sdist) as tar:
        shipped = {name.partition("/")[2] for name in tar.getnames()}
    if missing := sorted(set(tracked) - shipped):
        msg = f"{sdist.name} misses {', '.join(missing)}"
        raise SystemExit(msg)
    print(f"{sdist.name} contains the tracked source, tests, docs and Cargo files", flush=True)


def _runs_on_its_own(wheel: Path) -> None:
    subprocess.check_call(["uv", "pip", "install", "--python", sys.executable, "--no-index", str(wheel)])
    subprocess.check_call([str(Path(sys.executable).parent / "simplint"), "--version"])
    print(f"{wheel.name} installed without a package index and reported its version", flush=True)


def _ships_a_suite_that_compiles(sdist: Path, at: Path) -> None:
    # CI runs the tests from the checkout. Here we check that a packager can compile the suite from the sdist.
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
