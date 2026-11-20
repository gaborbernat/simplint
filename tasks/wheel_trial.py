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
    destination = work / "sources" / package.name
    with tarfile.open(archive) as source:
        source.extractall(destination, filter="data")
    return next(destination.iterdir()).resolve()


def executable(wheel: Path, directory: Path, name: str) -> Path:
    with ZipFile(wheel) as archive:
        (member,) = (
            entry for entry in archive.namelist() if entry.endswith((f"/scripts/{name}", f"/scripts/{name}.exe"))
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
            "control": executable(control, work / "control", "simplint"),
            "candidate": executable(wheel, work / "candidate", "simplint"),
        }
        suffix = wheel.name.split("-py3-none-")[1]
        with urllib.request.urlopen("https://pypi.org/pypi/simplint/0.14.2/json", timeout=60) as response:
            (published,) = (item for item in json.load(response)["urls"] if item["filename"].endswith("-" + suffix))
        baseline = download(Pinned(url=published["url"], sha256=published["digests"]["sha256"]), work / "wheels")
        binaries["published"] = executable(baseline, work / "published", "simplint")
        ruff_suffix = {
            "manylinux_2_28_aarch64.whl": "manylinux_2_17_aarch64.manylinux2014_aarch64.whl",
            "manylinux_2_28_x86_64.whl": "manylinux_2_17_x86_64.manylinux2014_x86_64.whl",
        }.get(suffix, suffix)
        with urllib.request.urlopen("https://pypi.org/pypi/ruff/0.17.4/json", timeout=60) as response:
            (ruff,) = (item for item in json.load(response)["urls"] if item["filename"].endswith("-" + ruff_suffix))
        ruff_wheel = download(Pinned(url=ruff["url"], sha256=ruff["digests"]["sha256"]), work / "wheels")
        binaries["ruff"] = executable(ruff_wheel, work / "ruff", "ruff")
        emulated = "riscv64" in suffix
        repeats = 1 if emulated else 8
        totals = {name: [0.0] * repeats for name in binaries}
        writer = csv.writer(stream)
        writer.writerow(("package", "repeat", "binary", "seconds", "status", "sha256"))
        packages = list(_TOP)
        random.Random(719).shuffle(packages)
        with ThreadPoolExecutor(max_workers=4) as pool:
            roots = list(pool.map(unpack, packages, [work] * len(packages)))
        env = os.environ | {"RAYON_NUM_THREADS": "4", "NO_COLOR": "1", "XDG_CONFIG_HOME": str(work / "config")}
        flags = [
            "check",
            "--isolated",
            "--target-version",
            "py311",
            "--no-fix",
            "--no-respect-gitignore",
            "--config",
            'include = ["*.py"]',
        ]
        with Path("tasks/wheel_inputs.csv").open(encoding="utf-8") as fingerprints:
            expected = dict(csv.reader(fingerprints))
        count = 0
        for index, (package, root) in enumerate(zip(packages, roots, strict=True)):
            for name, binary in binaries.items():
                files = subprocess.check_output([binary, *flags, "--show-files", "."], cwd=root, env=env)
                paths = sorted(Path(line.decode("utf-8")).relative_to(root).as_posix() for line in files.splitlines())
                fingerprint = hashlib.sha256("\n".join(paths).encode()).hexdigest()
                if fingerprint != expected[package.name]:
                    raise ValueError((package.name, name, len(paths), fingerprint))
                if name == "candidate":
                    count += len(paths)
            for rep in range(-1, repeats):
                names = list(binaries) if rep >= 0 else ["control", "candidate"]
                shift = (index + rep) % len(names)
                if rep >= len(names):
                    names.reverse()
                digests = {}
                with TemporaryDirectory(dir=work) as cache:
                    for name in names[shift:] + names[:shift]:
                        options = (
                            ["--select", "ALL", "--unsafe-fixes", "--output-format", "json"]
                            if rep < 0
                            else ["--output-format", "concise"]
                        )
                        run_env = (
                            env | {"_RJEM_MALLOC_CONF": "tcache_gc_incr_bytes:524288"} if name == "candidate" else env
                        )
                        start = time.perf_counter()
                        result = subprocess.run(
                            [binaries[name], *flags, *options, "--cache-dir", str(Path(cache) / name), "."],
                            cwd=root,
                            env=run_env,
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
        for name in ("control", "published", "ruff"):
            gains = [100 * (1 - new / old) for new, old in zip(totals["candidate"], totals[name], strict=True)]
            lines.append(f"vs {name}: median {statistics.median(gains):.2f}% paired, {gains}")
        Path("totals.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
        print(*lines, sep="\n", flush=True)


if __name__ == "__main__":
    measure()
