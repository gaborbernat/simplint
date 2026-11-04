# /// script
# requires-python = ">=3.12"
# ///
"""Pin the sdists simplint's profile-guided optimization trains on, and record the profile from them."""

from __future__ import annotations

import argparse
import hashlib
import http.client
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import time
import tomllib
import urllib.request
from collections import Counter
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING, Final, TypedDict, TypeVar

if TYPE_CHECKING:
    from collections.abc import Callable, Generator

_ROOT: Final[Path] = Path(__file__).resolve().parents[1]
#: The pinned sdists the training runs on.
_MANIFEST: Final[Path] = _ROOT / "tasks" / "pgo_corpus.toml"
#: Different shapes of code: a formatter, a package installer and an SDK core. A profile from the 532 projects of the
#: validation corpus made the binary no faster than one from these three, at 140 times the training time.
PROJECTS: Final[tuple[str, ...]] = ("black", "botocore", "pip")
#: simplint lints Python modules and reads stubs for the names they declare; nothing else in a project matters to it.
_PYTHON: Final[tuple[str, ...]] = (".py", ".pyi")
#: What the optimized build prints, given ``-Cllvm-args=-pgo-warn-missing-function``, for a function the profile has no
#: counts for, or counts of other code. Cargo hides the warnings of dependencies, so they all come from simplint's own
#: codegen units, which also hold the instances of its dependencies' generic functions.
_PGO_WARNING: Final = re.compile(
    r"^warning: \S+: (no profile data available for function|function [a-z ]+ change detected \([a-z ]+\)) (\S+)",
    re.MULTILINE,
)
#: The first crate root of a v0 mangled symbol, the crate defining the function: ``C``, an optional ``s<base62>_``
#: disambiguator, and the length of the name that follows.
_CRATE_ROOT: Final = re.compile(r"C(?:s[0-9A-Za-z]*_)?(\d+)_?")
#: Cargo colors its output when the toolchain action sets CARGO_TERM_COLOR.
_ANSI: Final = re.compile(r"\x1b\[[0-9;]*m")
#: The crates of the hot path. With a matching profile none of their functions lack data; cold startup code, such as
#: fs_err, argfile or ruff_command_line, may, and does not matter.
_GUARDED: Final[frozenset[str]] = frozenset({"ruff_linter", "ruff_python_parser", "simplint"})
_T = TypeVar("_T")


@dataclass(frozen=True, slots=True, kw_only=True)
class Sdist:
    # A source distribution on PyPI, pinned by its digest.
    url: str
    sha256: str

    @property
    def filename(self) -> str:
        # The archive name, the last segment of its URL.
        return self.url.rsplit("/", 1)[-1]


def main(arguments: list[str]) -> None:
    # Run the subcommand the command line names.
    #
    # :param arguments: the command line, without the program name
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    pin = commands.add_parser("manifest", help="pin the latest sdist of each training project")
    record = commands.add_parser("train", help="run an instrumented simplint over the sdists and merge its profile")
    record.add_argument("binary", type=Path, help="simplint built with -Cprofile-generate")
    record.add_argument(
        "work", type=Path, help="gets corpus/, profiles/ and simplint.profdata; the first two must be new"
    )
    record.add_argument("--cache", type=Path, help="keeps verified downloads between runs, <work>/downloads by default")
    guard = commands.add_parser("verify", help="fail if the optimized build found no profile data for hot code")
    guard.add_argument("log", type=Path, help="the output of the build with -Cllvm-args=-pgo-warn-missing-function")
    guard.add_argument("profile", type=Path, help="the merged profile the build used")
    for command in (pin, record):
        command.add_argument("--manifest", type=Path, default=_MANIFEST, help="the sdist manifest")
    options = parser.parse_args(arguments, namespace=_Options())
    if options.command == "train":
        _train(options.binary, options.work, options.cache or options.work / "downloads", options.manifest)
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
    log: Path


def pin_sdist(name: str) -> Sdist:
    # Pin the ``.tar.gz`` sdist of a project's latest release.
    #
    # :param name: the PyPI project
    # :returns: the pinned sdist
    # :raises ValueError: if the release ships no ``.tar.gz`` sdist
    url = f"https://pypi.org/pypi/{name}/json"
    for file in _retried(lambda: _release_files(url), url):
        if file["packagetype"] == "sdist" and file["filename"].endswith(".tar.gz"):
            return Sdist(url=file["url"], sha256=file["digests"]["sha256"])
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


# Retry what the network or a server can fail transiently, backing off, and let the last failure through.
def _retried(action: Callable[[], _T], what: str) -> _T:
    attempts = 4
    for attempt in range(1, attempts):
        try:
            return action()
        except (OSError, http.client.HTTPException) as error:
            print(f"{what}: {error}, retrying", file=sys.stderr, flush=True)
            time.sleep(2**attempt)
    return action()


def write_manifest(path: Path, sdists: list[Sdist]) -> None:
    # Write the manifest, one sdist per line.
    #
    # :param path: the TOML file to write
    # :param sdists: the source distributions
    lines = [
        "# The sdists tasks/pgo.py trains simplint's profile-guided optimization on, from its PROJECTS. Generated by",
        "# `python tasks/pgo.py manifest`, so regenerate rather than edit.",
        "sdists = [",
        *(f"  {{ url = {json.dumps(sdist.url)}, sha256 = {json.dumps(sdist.sha256)} }}," for sdist in sdists),
        "]",
    ]
    path.write_text("".join(f"{line}\n" for line in lines), encoding="utf-8")


def read_manifest(path: Path) -> list[Sdist]:
    # Read the manifest.
    #
    # :param path: the TOML file to read
    # :returns: the source distributions
    with path.open("rb") as stream:
        return [Sdist(**entry) for entry in tomllib.load(stream)["sdists"]]


def _train(binary: Path, work: Path, cache: Path, manifest: Path) -> Path:
    # Record a profile: run the instrumented simplint in check and fix mode over the sdists, then merge the raw
    # profiles.
    #
    # The merge uses the active Rust toolchain's llvm-profdata (component llvm-tools-preview), whose LLVM must match the
    # compiler that reads the profile. The corpus and profiles directories must not exist yet, so a stale run cannot
    # leak in.
    #
    # :param binary: simplint built with ``-Cprofile-generate``
    # :param work: the directory to extract and record into
    # :param cache: the directory keeping verified downloads
    # :param manifest: the sdist manifest
    # :returns: the merged profile
    # :raises subprocess.CalledProcessError: if simplint fails to finish a run
    profdata = llvm_profdata()
    corpus, profiles = work / "corpus", work / "profiles"
    profiles.mkdir(parents=True)
    corpus.mkdir()
    for sdist in read_manifest(manifest):
        extract(download(sdist, cache), corpus)
    environment = {**os.environ, "LLVM_PROFILE_FILE": str(profiles / "simplint-%m-%p.profraw")}
    check = [str(binary), "check", "--no-cache", "--config", str(_ROOT / "tasks" / "pgo.toml")]
    for flags in ([], ["--fix", "--unsafe-fixes", "--diff"]):
        command = [*check, *flags, str(corpus)]
        with _step(" ".join(["simplint check", *flags])):
            status = subprocess.run(command, stdout=subprocess.DEVNULL, env=environment, check=False).returncode
        # Exit code 1 only reports violations or pending fixes; anything above is a failed run.
        if status > 1:
            raise subprocess.CalledProcessError(status, command)
    merged = work / "simplint.profdata"
    subprocess.run([str(profdata), "merge", "--output", str(merged), str(profiles)], check=True)
    print(f"profile written to {merged}, {merged.stat().st_size / 2**20:.1f} MiB", flush=True)
    return merged


def _verify(log: Path, profile: Path) -> None:
    # Check that the profile matched the optimized build.
    #
    # A profile recorded from a build that differs in any compiler flag or Cargo profile setting, such as ``lto`` or
    # ``codegen-units``, names its functions under another crate disambiguator, so none of them match and the build
    # silently drops the whole gain. A matching profile can still lack a few generic instances the optimized build
    # creates and the instrumented one never did, such as drop glue; those name the same crate the profile holds.
    #
    # :param log: the output of the build with ``-Cllvm-args=-pgo-warn-missing-function``
    # :param profile: the merged profile the build used
    # :raises SystemExit: if the log shows no build of simplint, a changed function of a crate in GUARDED, or functions
    # of
    #     a crate in GUARDED that the profile holds under no matching crate disambiguator
    text = _ANSI.sub("", log.read_text(encoding="utf-8", errors="replace"))
    if not re.search(r"^\s*Compiling simplint ", text, re.MULTILINE):
        msg = f"::error::{log} shows no build of simplint, so it cannot tell whether the PGO profile matched"
        raise SystemExit(msg)
    warnings = [(kind, _crate_root(symbol)) for kind, symbol in _PGO_WARNING.findall(text)]
    missing = Counter(_crate(root) for _, root in warnings)
    print(f"functions without PGO profile data: {dict(missing.most_common()) or 'none'}", flush=True)
    guarded = [(kind, root) for kind, root in warnings if _crate(root) in _GUARDED]
    changed = {_crate(root) for kind, root in guarded if "change detected" in kind}
    if roots := {root for _, root in guarded if _crate(root) not in changed}:
        names = subprocess.run(
            [str(llvm_profdata()), "show", "--all-functions", str(profile)], capture_output=True, text=True, check=True
        ).stdout
        unmatched = {_crate(root) for root in roots if root not in names}
    else:
        unmatched = set()
    if hot := sorted(changed | unmatched):
        msg = f"::error::the PGO profile does not match this build, no profile data for functions of {', '.join(hot)}"
        raise SystemExit(msg)


# The crate root of a v0 mangled symbol, with its disambiguator, such as ``Cs1ORA0Rd0xIe_8simplint``; a symbol that is
# not v0 mangled stands for itself.
def _crate_root(symbol: str) -> str:
    if root := _CRATE_ROOT.search(symbol):
        return symbol[root.start() : root.end() + int(root[1])]
    return symbol


def _crate(root: str) -> str:
    if found := _CRATE_ROOT.match(root):
        return root[found.end() : found.end() + int(found[1])]
    return root


def llvm_profdata() -> Path:
    # Locate the active Rust toolchain's llvm-profdata.
    #
    # :returns: the executable
    # :raises FileNotFoundError: if the toolchain lacks the llvm-tools-preview component
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


def download(sdist: Sdist, folder: Path) -> Path:
    # Download a source distribution unless the folder holds it already, and check it against its pinned digest.
    #
    # :param sdist: the source distribution
    # :param folder: the download directory
    # :returns: the verified archive
    # :raises ValueError: if the download does not match its pinned sha256
    archive = folder / sdist.filename
    if archive.is_file() and _sha256(archive) == sdist.sha256:
        return archive
    folder.mkdir(parents=True, exist_ok=True)
    # Written aside and moved in place once verified, so an interrupted or tampered download never looks cached.
    partial = archive.with_name(f"{archive.name}.part")
    _retried(lambda: _save(sdist.url, partial), sdist.url)
    if (digest := _sha256(partial)) != sdist.sha256:
        partial.unlink()
        msg = f"{sdist.url}: sha256 {digest}, pinned {sdist.sha256}"
        raise ValueError(msg)
    partial.replace(archive)
    return archive


def _save(url: str, path: Path) -> None:
    with urllib.request.urlopen(url, timeout=60) as response, path.open("wb") as stream:
        shutil.copyfileobj(response, stream)


# Hashed here rather than by sha256sum, which escapes its output when a Windows path holds a backslash.
def _sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def extract(archive: Path, corpus: Path) -> None:
    # Extract the Python files of an sdist into its own directory under the corpus.
    #
    # A file the platform cannot hold, such as one with a name Windows reserves, is skipped with a warning rather than
    # failing the build over one file.
    #
    # :param archive: the source distribution
    # :param corpus: the directory to extract into
    destination = corpus / archive.name.removesuffix(".tar.gz")
    skipped = []
    with tarfile.open(archive, "r|gz") as tar:
        for member in tar:
            if member.isfile() and member.name.endswith(_PYTHON):
                try:
                    tar.extract(member, destination, filter="data")
                except (tarfile.FilterError, OSError) as error:
                    skipped.append(f"{member.name} ({error})")
    if skipped:
        print(f"::warning::{archive.name}: skipped {len(skipped)} Python files, first {skipped[0]}", flush=True)


if __name__ == "__main__":
    main(sys.argv[1:])


__all__ = [
    "Sdist",
    "download",
    "extract",
    "llvm_profdata",
    "main",
    "pin_sdist",
    "read_manifest",
    "write_manifest",
]
