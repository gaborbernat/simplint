"""Collect profile data for release builds using pinned Python source distributions."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tarfile
import time
import tomllib
import urllib.request
from collections import Counter
from contextlib import contextmanager
from pathlib import Path
from typing import TYPE_CHECKING, Final, TypedDict

from tasks.download import Pinned, download, retried

if TYPE_CHECKING:
    from collections.abc import Generator

_ROOT: Final[Path] = Path(__file__).resolve().parents[1]
#: The pinned sdists the training runs on.
_MANIFEST: Final[Path] = _ROOT / "tasks" / "pgo_corpus.toml"
#: Train on a formatter, an AWS SDK library and a package installer.
PROJECTS: Final[tuple[str, ...]] = ("black", "botocore", "pip")
#: LLVM reports missing counts and changed functions when the build enables -pgo-warn-missing-function.
#: simplint's codegen units can include generic functions from dependencies.
_PGO_WARNING: Final[re.Pattern[str]] = re.compile(
    r"""
    ^warning:[ ]\S+:[ ]  # Rust warning prefix
    (?P<kind>no[ ]profile[ ]data[ ]available[ ]for[ ]function|function[ ][a-z ]+[ ]change[ ]detected[ ]\([a-z ]+\))
    [ ](?P<symbol>\S+)   # mangled function name
    """,
    re.MULTILINE | re.VERBOSE,
)
#: Keep the disambiguator as well as the crate name to distinguish builds of the same crate.
_CRATE_ROOT: Final[re.Pattern[str]] = re.compile(
    r"""
    C                    # v0 crate root marker
    (?:s[0-9A-Za-z]*_)?  # optional disambiguator
    (?P<length>\d+)_?     # byte length of the crate name
    """,
    re.VERBOSE,
)
#: Cargo colors its output when the toolchain action sets CARGO_TERM_COLOR.
_ANSI: Final[re.Pattern[str]] = re.compile(r"\x1b\[[0-9;]*m")
#: Missing counts in startup code alone should not fail verification.
_GUARDED: Final[frozenset[str]] = frozenset({"ruff_linter", "ruff_python_parser", "simplint"})


def main(arguments: list[str]) -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    pin = commands.add_parser("manifest", help="pin the latest sdist of each training project")
    record = commands.add_parser("train", help="run an instrumented simplint over the sdists and merge its profile")
    record.add_argument("binary", type=Path, help="simplint built with -Cprofile-generate")
    record.add_argument(
        "work", type=Path, help="gets corpus/, profiles/ and simplint.profdata; the first two must be new"
    )
    record.add_argument("--cache", type=Path, help="keeps verified downloads between runs, <work>/downloads by default")
    fetch = commands.add_parser("download", help="download the sdists into the cache the release builds restore")
    fetch.add_argument("into", type=Path, help="the cache directory to download into")
    fetch.add_argument("--extract", type=Path, help="unpack the pinned Python sources into this directory")
    guard = commands.add_parser("verify", help="fail if the optimized build found no profile data for hot code")
    guard.add_argument("log", type=Path, help="the output of the build with -Cllvm-args=-pgo-warn-missing-function")
    guard.add_argument("profile", type=Path, help="the merged profile the build used")
    for command in (pin, record, fetch):
        command.add_argument("--manifest", type=Path, default=_MANIFEST, help="the sdist manifest")
    options = parser.parse_args(arguments, namespace=_Options())
    if options.command == "train":
        _train(options.binary, options.work, options.cache or options.work / "downloads", options.manifest)
        return
    if options.command == "download":
        for sdist in read_manifest(options.manifest):
            archive = download(sdist, options.into)
            if options.extract is not None:
                extract(archive, options.extract)
        return
    if options.command == "verify":
        _verify(options.log, options.profile)
        return
    write_manifest(options.manifest, [pin_sdist(name) for name in PROJECTS])


class _Options(argparse.Namespace):
    command: str
    manifest: Path
    binary: Path
    work: Path
    cache: Path | None
    into: Path
    extract: Path | None
    log: Path


def pin_sdist(name: str) -> Pinned:
    # Pin the ``.tar.gz`` sdist of a project's latest release.
    url = f"https://pypi.org/pypi/{name}/json"
    for file in retried(lambda: _release_files(url), url):
        if file["packagetype"] == "sdist" and file["filename"].endswith(".tar.gz"):
            return Pinned(url=file["url"], sha256=file["digests"]["sha256"])
    msg = f"the latest release of {name} ships no .tar.gz sdist"
    raise ValueError(msg)


class _File(TypedDict):
    filename: str
    packagetype: str
    url: str
    digests: dict[str, str]


def _release_files(url: str) -> list[_File]:
    with urllib.request.urlopen(url, timeout=60) as response:
        files: list[_File] = json.load(response)["urls"]
    return files


def write_manifest(path: Path, sdists: list[Pinned]) -> None:
    lines = [
        "# The sdists tasks/pgo.py trains simplint's profile-guided optimization on, from its PROJECTS. Generated by",
        "# `python -m tasks.pgo manifest`, so regenerate rather than edit.",
        "sdists = [",
        *(f"  {{ url = {json.dumps(sdist.url)}, sha256 = {json.dumps(sdist.sha256)} }}," for sdist in sdists),
        "]",
    ]
    path.write_text("".join(f"{line}\n" for line in lines), encoding="utf-8")


def read_manifest(path: Path) -> list[Pinned]:
    with path.open("rb") as stream:
        return [Pinned(**entry) for entry in tomllib.load(stream)["sdists"]]


def _train(binary: Path, work: Path, cache: Path, manifest: Path) -> None:
    # Record a profile: run the instrumented simplint over the sdists in check and fix mode without a cache, then twice
    # with one, and merge the raw profiles.
    #
    # The merge uses the active Rust toolchain's llvm-profdata (component llvm-tools-preview), whose LLVM must match the
    # compiler that reads the profile. The corpus and profiles directories must not exist yet, so a stale run cannot
    # leak in.
    profdata = llvm_profdata()
    corpus, profiles = work / "corpus", work / "profiles"
    profiles.mkdir(parents=True)
    corpus.mkdir()
    for sdist in read_manifest(manifest):
        extract(download(sdist, cache), corpus)
    environment = {**os.environ, "LLVM_PROFILE_FILE": str(profiles / "simplint-%m-%p.profraw")}
    check = [str(binary), "check", "--config", str(_ROOT / "tasks" / "pgo.toml")]
    cached = ["--cache-dir", str(work / "cache")]
    # The first cached run fills the cache and the second trains the warm path, which reads every result back from it.
    for flags in (["--no-cache"], ["--no-cache", "--fix", "--unsafe-fixes", "--diff"], cached, cached):
        command = [*check, *flags, str(corpus)]
        with _step(" ".join(["simplint check", *flags])):
            status = subprocess.run(command, stdout=subprocess.DEVNULL, env=environment, check=False).returncode
        # Exit code 1 reports violations or pending fixes. A signal gives a negative return code on POSIX.
        if status not in {0, 1}:
            raise subprocess.CalledProcessError(status, command)
    merged = work / "simplint.profdata"
    subprocess.run([str(profdata), "merge", "--output", str(merged), str(profiles)], check=True)
    print(f"profile written to {merged}, {merged.stat().st_size / 2**20:.1f} MiB", flush=True)


_COMPILING_SIMPLINT_PATTERN: Final[re.Pattern[str]] = re.compile(r"^\s*Compiling simplint ", re.MULTILINE)


def _verify(log: Path, profile: Path) -> None:
    # A different build can give the same crate a different disambiguator. A missing generic instance can be
    # legitimate when other functions from the same crate and build have counts.
    text = _ANSI.sub("", log.read_text(encoding="utf-8", errors="replace"))
    if not _COMPILING_SIMPLINT_PATTERN.search(text):
        msg = f"::error::{log} shows no build of simplint, so it cannot tell whether the PGO profile matched"
        raise SystemExit(msg)
    warnings = [(kind, _crate_root(symbol)) for kind, symbol in _PGO_WARNING.findall(text)]
    missing = Counter(_crate(root) for _, root in warnings)
    print(f"functions without PGO profile data: {dict(missing.most_common()) or 'none'}", flush=True)
    guarded = [(kind, root) for kind, root in warnings if _crate(root) in _GUARDED]
    changed = {_crate(root) for kind, root in guarded if "change detected" in kind}
    if hot := sorted(changed | _unmatched({root for _, root in guarded if _crate(root) not in changed}, profile)):
        msg = f"::error::the PGO profile does not match this build, no profile data for functions of {', '.join(hot)}"
        raise SystemExit(msg)


def _unmatched(roots: set[str], profile: Path) -> set[str]:
    if not roots:
        return set()
    names = subprocess.run(
        [str(llvm_profdata()), "show", "--all-functions", str(profile)], capture_output=True, text=True, check=True
    ).stdout
    return {_crate(root) for root in roots if root not in names}


# The crate root of a v0 mangled symbol, with its disambiguator, such as ``Cs1ORA0Rd0xIe_8simplint``; a symbol that is
# not v0 mangled stands for itself.
def _crate_root(symbol: str) -> str:
    if root := _CRATE_ROOT.search(symbol):
        return symbol[root.start() : root.end() + int(root["length"])]
    return symbol


def _crate(root: str) -> str:
    if found := _CRATE_ROOT.match(root):
        return root[found.end() : found.end() + int(found["length"])]
    return root


def llvm_profdata() -> Path:
    verbose = _rustc("--version", "--verbose").splitlines()
    host = next(line.removeprefix("host: ") for line in verbose if line.startswith("host: "))
    name = "llvm-profdata.exe" if "windows" in host else "llvm-profdata"
    tool = Path(_rustc("--print", "sysroot").strip()) / "lib" / "rustlib" / host / "bin" / name
    if not tool.is_file():
        msg = f"missing {tool}, run: rustup component add llvm-tools-preview"
        raise FileNotFoundError(msg)
    return tool


# From the project root, where rust-toolchain.toml selects the toolchain that builds simplint.
def _rustc(*arguments: str) -> str:
    return subprocess.run(["rustc", *arguments], cwd=_ROOT, capture_output=True, text=True, check=True).stdout


@contextmanager
def _step(title: str) -> Generator[None]:
    print(f"::group::{title}", flush=True)
    start = time.monotonic()
    try:
        yield
    finally:
        print(f"{title}: {time.monotonic() - start:.0f}s", flush=True)
        print("::endgroup::", flush=True)


def extract(archive: Path, corpus: Path) -> None:
    # Extract the Python files of an sdist into its own directory under the corpus.
    #
    # A file the platform cannot hold, such as one with a name Windows reserves, is skipped with a warning rather than
    # failing the build over one file.
    destination = corpus / archive.name.removesuffix(".tar.gz")
    skipped = []
    with tarfile.open(archive, "r|gz") as tar:
        for member in tar:
            # Limit the training corpus to Python source and stubs.
            if member.isfile() and member.name.endswith((".py", ".pyi")):
                try:
                    tar.extract(member, destination, filter="data")
                except (tarfile.FilterError, OSError) as error:
                    skipped.append(f"{member.name} ({error})")
    if skipped:
        print(f"::warning::{archive.name}: skipped {len(skipped)} Python files, first {skipped[0]}", flush=True)


if __name__ == "__main__":
    main(sys.argv[1:])


__all__ = [
    "extract",
    "llvm_profdata",
    "main",
    "pin_sdist",
    "read_manifest",
    "write_manifest",
]
