"""Check simplint against the most downloaded PyPI projects, so a rule or fix that breaks real code shows up."""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import re
import shutil
import subprocess
import sys
import tarfile
import urllib.request
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Final


@dataclass(frozen=True, slots=True, kw_only=True)
class Package:
    """A pinned source distribution."""

    name: str
    version: str
    sha256: str

    @property
    def url(self) -> str:
        """PyPI's stable address for the archive, which redirects to where the file is stored."""
        return f"https://files.pythonhosted.org/packages/source/{self.name[0]}/{self.name}/{self.filename}"

    @property
    def filename(self) -> str:
        """The archive name PyPI gives the source distribution."""
        return f"{self.name.replace('-', '_')}-{self.version}.tar.gz"


#: The top of https://hugovk.github.io/top-pypi-packages/ on 2026-10-01, pinned so a run is reproducible.
PACKAGES: Final[tuple[Package, ...]] = (
    Package(
        name="boto3", version="1.43.122", sha256="c923650d09e4c445564ca14d5ce89466c3083884b6f6c189f0f445fc17f02eb5"
    ),
    Package(
        name="packaging", version="26.3", sha256="94edc256424af38762eb31306eed28beb9f0efc50a8837492c9d6fd6004aed79"
    ),
    Package(
        name="typing-extensions",
        version="4.16.0",
        sha256="dc983d19a509c94dba722ee6abd33940f7c05a89e243c47e907eb4db6f1a43e5",
    ),
    Package(name="idna", version="3.20", sha256="a7db850025b95ded1eae8a46181a1a6c56c92c96f0e2b005d9ff8dc0210cab44"),
    Package(name="urllib3", version="2.8.0", sha256="63bf2ead4c879426ebf22ef2a781eeb4aa3b4ae798a0435506f8687fd5bb9b63"),
    Package(
        name="certifi", version="2026.10.23", sha256="a9895e6e2758b371e0545e321a3a8fd1fcfd9829ada8824a936dfb16aaaf36cd"
    ),
    Package(
        name="requests", version="2.34.2", sha256="f288924cae4e29463698d6d60bc6a4da69c89185ad1e0bcc4104f584e960b9ed"
    ),
    Package(
        name="charset-normalizer",
        version="3.5.2",
        sha256="39de2a259fc954455c57274dc94c79d5842774e1247a016aff30bc0efed0f4ef",
    ),
    Package(
        name="cryptography", version="50.0.2", sha256="7b46165bb56eb4704e2eaaf86f3c940d19154535d9b0ca7d6d590b04060e00d5"
    ),
    Package(
        name="setuptools", version="84.2.1", sha256="3ff8b3b5492a0aa66960f4419365a86b84e013070d4313e4d4a6894f5d64d6bc"
    ),
    Package(name="cffi", version="2.1.1", sha256="dd31f52ea1086513bb9df30f8fcee9b8918323ae067a3d5b78bc826a000712be"),
    Package(
        name="pygments", version="2.21.0", sha256="610ca751c9bc2492b38eb9a38a7fbc93edbbb2d7182edaf34e66ae493dee5c8c"
    ),
    Package(name="pyyaml", version="6.0.3", sha256="d76623373421df22fb4cf8817020cbb7ef15c725b9d5e45f17e189bfc384190f"),
    Package(
        name="python-dateutil",
        version="3.0.0",
        sha256="7ebcb3c29a92f3ceafe79a7a48a5c45677e9e004bbeec5742ac0bbb00560b7a2",
    ),
    Package(name="six", version="1.17.0", sha256="ff70335d468e7eb6ec65b95b99d3a2836546063f63acc5171de367e834932a81"),
    Package(name="pluggy", version="1.7.0", sha256="d1eaa46ebb595891b860ab086b4d09c8588af65ebd4361b8e8f4bb8920b90ba8"),
    Package(
        name="botocore", version="1.43.122", sha256="2fc53b68b35314b1738cad46e6170d0556496863913451428fc3afccb8c7fa95"
    ),
    Package(
        name="pydantic", version="2.14.2", sha256="fb7b2db6cefe8414dd612edeede49c4c433acb0432b0f92b95024e45f66c505d"
    ),
    Package(
        name="pycparser", version="3.12", sha256="f03387877eab82ce01bbdfd0ba7252ee55a90b1c708d5129e2ce915475cf1db6"
    ),
    Package(name="click", version="8.5.0", sha256="ba0d2089de75ea0310e2dde03160e6ca10009947fb95a182f9b54021bb272e34"),
    Package(
        name="pydantic-core",
        version="2.50.2",
        sha256="1b0e8dc3932cf4e9ed5727570325cacc716b9409ffc794a221c0ccc72e8eb775",
    ),
    Package(name="anyio", version="4.15.1", sha256="9f28306018cbd6d329e64a36d58256edff76dd996fe423bc957326e578b82a94"),
    Package(name="numpy", version="2.5.4", sha256="9a94cf751c9ad8ebaa835bcd3d40dacf8534ad086b88c38029b65123c7999d2a"),
    Package(name="pytest", version="9.2.0", sha256="97c4f3b1f3c7414aa36465e09b8220f1d03627993bd51b1282e193753c80557b"),
    Package(
        name="iniconfig", version="2.3.1", sha256="67f4b9c50da0dedf52af349e7749a80a9057a5031199791b906c3bb3ae878960"
    ),
)


def main(arguments: list[str]) -> int:
    """
    Lint and fix each pinned project, and report what went wrong.

    :param arguments: the command line, without the program name
    :returns: the exit status, 1 when any project has a problem
    :raises ValueError: if a name is not pinned or a download does not match its hash
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cache", type=Path, default=Path(".tox") / "ecosystem", help="keeps the downloads")
    parser.add_argument("--simplint", default="simplint", help="the simplint executable to check")
    parser.add_argument("names", nargs="*", help="projects to check, all when none are given")
    options = parser.parse_args(arguments, namespace=_Options())
    names = options.names
    if unknown := set(names) - {package.name for package in PACKAGES}:
        msg = f"not pinned: {', '.join(sorted(unknown))}"
        raise ValueError(msg)
    failed = False
    for package in PACKAGES:
        if names and package.name not in names:
            continue
        problems = check(package, download(package, options.cache), options.simplint)
        failed |= bool(problems)
        print(f"{package.name} {package.version}: {'; '.join(problems) or 'ok'}", flush=True)
    return int(failed)


class _Options(argparse.Namespace):
    cache: Path
    simplint: str
    names: list[str]


def download(package: Package, cache: Path) -> Path:
    """
    Fetch a source distribution into the cache unless it is already there.

    :param package: the distribution to fetch
    :param cache: the download directory
    :returns: the path of the verified archive
    :raises ValueError: if the archive does not match its pinned hash
    """
    archive = cache / package.filename
    if not archive.is_file():
        cache.mkdir(parents=True, exist_ok=True)
        with urllib.request.urlopen(package.url) as response:  # a pinned https URL
            archive.write_bytes(response.read())
    if hashlib.sha256(archive.read_bytes()).hexdigest() != package.sha256:
        archive.unlink()
        msg = f"{archive.name} does not match its pinned sha256"
        raise ValueError(msg)
    return archive


def check(package: Package, archive: Path, simplint: str) -> list[str]:
    """
    Run the documented pipeline on a project and check that one round of it reaches a fixed point.

    The pipeline is ``ruff check --fix --unsafe-fixes``, ``simplint check --fix --unsafe-fixes`` and ``ruff format``.
    The baseline runs the same commands without simplint.

    :param package: the project, for its name in messages
    :param archive: its source distribution
    :param simplint: the simplint executable
    :returns: one message per problem found, empty when the project passes
    """
    with TemporaryDirectory(prefix=f"{package.name}-") as folder:
        unpacked = Path(folder) / "original"
        with tarfile.open(archive) as tar:
            tar.extractall(unpacked, filter="data")
        # Run from the project's own directory, as a user would: Ruff, and simplint with it, look for the
        # `requires-python` that sets the target version from where it runs.
        original = next(unpacked.iterdir())
        fixed = Path(folder) / "fixed" / original.name
        shutil.copytree(original, fixed)
        report = ["--output-format", "concise", "."]
        ruff_fix = ["ruff", "check", "--fix", "--unsafe-fixes", "--exit-zero", "--quiet", "."]
        simplint_fix = [simplint, "check", "--fix-only", "--unsafe-fixes", *report]
        for tree in (original, fixed):
            _run(ruff_fix, tree)
        # A file Ruff cannot read either, such as one in another encoding, is not simplint's failure.
        unreadable = set(_io_errors(_run(["ruff", "check", "--exit-zero", *report], fixed)))
        problems = [f"check: {line}" for line in _failures(_run([simplint, "check", *report], fixed), unreadable)]
        problems += [f"fix: {line}" for line in _failures(_run(simplint_fix, fixed), unreadable)]
        # Rules Ruff warns conflict with its formatter for this project cannot hold after `ruff format`, with or without
        # simplint, as Ruff documents; their findings say nothing about simplint.
        conflicting = re.findall(r"\(`([A-Z]+[0-9]+)`\)", _run(["ruff", "format", "--check", "."], original).stderr)
        unaffected = ["--ignore", ",".join(conflicting)] if conflicting else []
        # Formatting splits a line and can leave its pragma comment after the closing bracket, where it no longer
        # covers the line it was written for; that happens to the project with or without simplint, as Ruff matches
        # suppressions the same way.
        stranded_before = _stranded_pragmas(original)
        for tree in (original, fixed):
            _run(["ruff", "format", "--quiet", "."], tree)
        stranding = {
            path for path, count in _stranded_pragmas(original).items() if count > stranded_before.get(path, 0)
        }
        problems += [f"{path} no longer parses" for path in _broken(original, fixed)]
        # The project's own settings give the target version, and so whether annotations are evaluated lazily.
        undefined = ["--select", "F821,F811,F823", "--ignore-noqa", "--config", "lint.per-file-ignores = {}"]
        problems += [f"{path}: new {code}" for path, code in _new_findings(original, fixed, undefined)]
        problems += [f"{path}: new {code}" for path, code in _new_findings(original, fixed, unaffected)]
        problems += _second_round(simplint, (original, fixed), unaffected, stranding)
        return problems


def _run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, cwd=cwd, capture_output=True, text=True, check=False)


# Ruff names the rule `E902`, or `io-error` when preview shows rules by name.
def _io_errors(result: subprocess.CompletedProcess[str]) -> list[str]:
    return [line for line in result.stdout.splitlines() if re.search(r":\d+:\d+: (E902 |io-error: )", line)]


# As with `ruff check`, a report exits 1 and only a run that could not finish exits 2; a file simplint cannot read or a
# fix it had to revert shows up as an `io-error` report or an error on standard error.
def _failures(result: subprocess.CompletedProcess[str], unreadable: set[str]) -> list[str]:
    failures = [line for line in result.stderr.splitlines() if line.startswith(("error", "simplint failed"))]
    failures += [line for line in _io_errors(result) if line not in unreadable]
    if result.returncode > 1 and not failures:
        failures.append(result.stderr.strip())
    return failures


def _stranded_pragmas(root: Path) -> dict[str, int]:
    pattern = re.compile(r"^\s*[)\]}]+[,:]?\s*#\s*(noqa|type:|pyright:|ruff:|simplint:)", re.MULTILINE)
    return {
        path.relative_to(root).as_posix(): len(pattern.findall(path.read_text(errors="replace")))
        for path in root.rglob("*.py")
    }


def _broken(original: Path, fixed: Path) -> list[str]:
    broken = []
    for path in sorted(original.rglob("*.py")):
        relative = path.relative_to(original)
        if _parses(path) and not _parses(fixed / relative):
            broken.append(relative.as_posix())
    return broken


def _parses(path: Path) -> bool:
    try:
        ast.parse(path.read_bytes())
    except SyntaxError:
        return False
    return True


# Renames move a name between lines, so findings are compared by count per file and code rather than by location.
def _new_findings(original: Path, fixed: Path, options: list[str]) -> list[tuple[str, str]]:
    before, after = _findings(original, options), _findings(fixed, options)
    return sorted(key for key, count in after.items() if count > before[key])


def _findings(root: Path, options: list[str]) -> Counter[tuple[str, str]]:
    command = ["ruff", "check", *options, "--exit-zero", "--output-format", "json", "."]
    report = json.loads(_run(command, root).stdout)
    return Counter(
        (Path(item["filename"]).relative_to(root.resolve()).as_posix(), item["code"] or "invalid-syntax")
        for item in report
    )


# What a second round of the pipeline changes in the fixed tree, which it must not.
def _second_round(simplint: str, trees: tuple[Path, Path], unaffected: list[str], stranding: set[str]) -> list[str]:
    original, fixed = trees
    problems = []
    # Ruff alone does not always settle in one round, so its second round counts only where it settles the baseline.
    ruff_again = ["ruff", "check", "--fix-only", "--unsafe-fixes", "--diff", "--quiet", *unaffected, "."]
    if _run(ruff_again, fixed).returncode == 1 and _run(ruff_again, original).returncode == 0:
        problems.append("a second round of ruff check --fix changed the code")
    diff = _run([simplint, "check", "--unsafe-fixes", "--diff", "."], fixed).stdout
    if set(re.findall(r"^--- (\S+)$", diff, re.MULTILINE)) - stranding:
        problems.append("a second round of simplint check --fix changed the code")
    if _run(["ruff", "format", "--check", "--quiet", "."], fixed).returncode == 1:
        problems.append("ruff format left the code unformatted")
    return problems


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))


__all__ = [
    "PACKAGES",
    "Package",
    "check",
    "download",
    "main",
]
