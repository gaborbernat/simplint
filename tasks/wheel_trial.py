"""Compare RISC-V diagnostics and fixes with a native build."""

from __future__ import annotations

import csv
import hashlib
import os
import subprocess
import tarfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from tempfile import TemporaryDirectory
from zipfile import ZipFile

from tasks.download import Pinned, download
from tasks.ecosystem import _TOP, Package


def unpack(package: Package, work: Path) -> Path:
    archive = download(Pinned(url=package.url, sha256=package.sha256), work / "archives")
    destination = work / package.name
    with tarfile.open(archive) as source:
        source.extractall(destination, filter="data")
    return next(destination.iterdir())


def executable(wheel: Path, directory: Path, name: str) -> Path:
    with ZipFile(wheel) as archive:
        (member,) = (
            entry for entry in archive.namelist() if entry.endswith((f"/scripts/{name}", f"/scripts/{name}.exe"))
        )
        binary = Path(archive.extract(member, directory)).resolve()
    binary.chmod(0o755)
    return binary


def main() -> None:
    with TemporaryDirectory() as temporary, Path("correctness.csv").open("w", newline="", encoding="utf-8") as stream:
        work = Path(temporary)
        (wheel,) = Path("dist").glob("*.whl")
        binaries = (Path("reference/release/simplint").resolve(), executable(wheel, work / "emulated", "simplint"))
        with Path("tasks/wheel_inputs.csv").open(encoding="utf-8") as fingerprints:
            expected = dict(csv.reader(fingerprints))
        env = os.environ | {"RAYON_NUM_THREADS": "4", "NO_COLOR": "1", "XDG_CONFIG_HOME": str(work / "config")}
        flags = [
            "check",
            "--isolated",
            "--target-version",
            "py311",
            "--no-cache",
            "--no-respect-gitignore",
            "--config",
            'include = ["*.py"]',
        ]
        writer = csv.writer(stream)
        writer.writerow(("package", "files", "status", "sha256"))
        count = 0
        with ThreadPoolExecutor(max_workers=4) as pool:
            for package, root in zip(_TOP, pool.map(unpack, _TOP, [work] * len(_TOP)), strict=True):
                results = []
                for binary in binaries:
                    files = subprocess.check_output([binary, *flags, "--show-files", "."], cwd=root, env=env)
                    paths = sorted(Path(line.decode()).relative_to(root).as_posix() for line in files.splitlines())
                    if hashlib.sha256("\n".join(paths).encode()).hexdigest() != expected[package.name]:
                        raise ValueError((package.name, binary, len(paths)))
                    result = subprocess.run(
                        [
                            binary,
                            *flags,
                            "--no-fix",
                            "--select",
                            "ALL",
                            "--unsafe-fixes",
                            "--output-format",
                            "json",
                            ".",
                        ],
                        cwd=root,
                        env=env,
                        capture_output=True,
                    )
                    if result.returncode not in (0, 1):
                        raise RuntimeError((package.name, binary, result.stderr))
                    results.append((len(paths), result.returncode, hashlib.sha256(result.stdout).hexdigest()))
                if results[0] != results[1]:
                    raise ValueError((package.name, results))
                writer.writerow((package.name, *results[0]))
                stream.flush()
                count += results[0][0]
        if count != 16150:
            raise ValueError(count)
        print(f"{len(_TOP)} packages, {count} Python files, all diagnostics and fixes match")


if __name__ == "__main__":
    main()
