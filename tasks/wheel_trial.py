"""Time paired cold checks on the release wheel corpus."""

from __future__ import annotations

import csv
import hashlib
import json
import os
import random
import statistics
import subprocess
import tarfile
import time
import urllib.request
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
        for member in source:
            if member.isfile() and (
                member.name.endswith((".py", ".pyi", ".toml"))
                or Path(member.name).name in {".gitignore", ".ignore", ".ruffignore", ".gitmodules"}
            ):
                source.extract(member, destination, filter="data")
    return next(destination.iterdir())


def executable(wheel: Path, directory: Path) -> Path:
    with ZipFile(wheel) as archive:
        (member,) = (
            name for name in archive.namelist() if name.endswith(("/scripts/simplint", "/scripts/simplint.exe"))
        )
        binary = Path(archive.extract(member, directory)).resolve()
    binary.chmod(0o755)
    return binary


def measure() -> None:
    with TemporaryDirectory() as temporary, Path("measurements.csv").open("w", newline="", encoding="utf-8") as stream:
        work = Path(temporary)
        (wheel,) = Path("dist").glob("*.whl")
        (control,) = Path("control").glob("*.whl")
        binaries = {
            "control": executable(control, work / "control"),
            "candidate": executable(wheel, work / "candidate"),
        }
        suffix = wheel.name.split("-py3-none-")[1]
        with urllib.request.urlopen("https://pypi.org/pypi/simplint/0.14.2/json", timeout=60) as response:
            (published,) = (item for item in json.load(response)["urls"] if item["filename"].endswith("-" + suffix))
        baseline = download(Pinned(url=published["url"], sha256=published["digests"]["sha256"]), work / "wheels")
        binaries["published"] = executable(baseline, work / "published")
        emulated = "riscv64" in suffix
        repeats = 1 if emulated else 6
        totals = {name: [0.0] * repeats for name in binaries}
        writer = csv.writer(stream)
        writer.writerow(("package", "repeat", "binary", "seconds", "status", "sha256"))
        packages = list(_TOP)
        random.Random(719).shuffle(packages)
        with ThreadPoolExecutor(max_workers=4) as pool:
            roots = list(pool.map(unpack, packages, [work] * len(packages)))
        env = os.environ | {"RAYON_NUM_THREADS": "4", "NO_COLOR": "1", "XDG_CONFIG_HOME": str(work / "config")}
        flags = ["check", "--isolated", "--target-version", "py311", "--no-fix", "--config", 'include = ["*.py"]']
        count = 0
        for index, (package, root) in enumerate(zip(packages, roots, strict=True)):
            files = subprocess.check_output([binaries["candidate"], *flags, "--show-files", "."], cwd=root, env=env)
            count += len(files.splitlines())
            for rep in range(-1, repeats):
                names = list(binaries) if rep >= 0 else ["control", "candidate"]
                shift = (index + rep) % len(names)
                if rep >= 3:
                    names.reverse()
                digests = {}
                with TemporaryDirectory(dir=work) as cache:
                    for name in names[shift:] + names[:shift]:
                        options = (
                            ["--select", "ALL", "--unsafe-fixes", "--output-format", "json"]
                            if rep < 0
                            else ["--output-format", "concise"]
                        )
                        start = time.perf_counter()
                        result = subprocess.run(
                            [binaries[name], *flags, *options, "--cache-dir", str(Path(cache) / name), "."],
                            cwd=root,
                            env=env,
                            capture_output=True,
                            check=False,
                        )
                        elapsed = time.perf_counter() - start
                        if result.returncode not in (0, 1):
                            raise RuntimeError((package.name, name, result.stderr))
                        digests[name] = (result.returncode, hashlib.sha256(result.stdout).hexdigest())
                        if rep >= 0:
                            totals[name][rep] += elapsed
                        writer.writerow((package.name, rep, name, elapsed, *digests[name]))
                    if digests["control"] != digests["candidate"]:
                        raise ValueError((package.name, rep, digests))
            stream.flush()
            if index % 10 == 0:
                print(package.name, flush=True)
        if count != 16150:
            raise ValueError(f"discovered {count} Python files")
        lines = [f"{suffix}: {count} Python files, emulated={emulated}", repr(totals)]
        for name in ("control", "published"):
            gains = [100 * (1 - new / old) for new, old in zip(totals["candidate"], totals[name], strict=True)]
            lines.append(f"vs {name}: median {statistics.median(gains):.2f}% paired, {gains}")
        Path("totals.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
        print(*lines, sep="\n", flush=True)


if __name__ == "__main__":
    measure()
