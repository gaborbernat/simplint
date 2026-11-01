"""Keep the workflow cargo-edit pins together."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence


def main(argv: Sequence[str] | None = None) -> None:
    """
    Print the next cargo-edit version or bump the workflow pins, as the command line asks.

    :param argv: the arguments to parse, ``sys.argv`` when omitted
    :raises ValueError: if the version list on stdin is not a list, or a pin is missing or invalid
    """
    parser = argparse.ArgumentParser(description="Check or update the cargo-edit workflow pin")
    parser.add_argument("command", choices=("candidate", "update"))
    parser.add_argument("version", nargs="?")
    parser.add_argument("--rust-version")
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args(argv)
    if args.command == "update":
        if args.version is None:
            parser.error("update requires a version")
        update(args.root, args.version)
    else:
        if args.rust_version is None:
            parser.error("candidate requires --rust-version")
        versions = json.load(sys.stdin)
        if not isinstance(versions, list):
            msg = "expected a crates.io version list"
            raise ValueError(msg)
        if selected := candidate(args.root, versions, args.rust_version):
            print(selected)


def candidate(root: Path, versions: list[dict[str, str | bool | None]], rust_version: str) -> str | None:
    """
    Pick the newest unyanked cargo-edit release newer than the pin that the Rust toolchain can build.

    :param root: the project root
    :param versions: the crates.io version entries
    :param rust_version: the available Rust version
    :returns: the chosen version, ``None`` when no release qualifies
    :raises ValueError: if the workflow pins are missing, disagree or the Rust version is invalid
    """
    current = version_parts(current_version(root))
    rust = version_parts(rust_version)
    compatible = []
    for entry in versions:
        version = entry.get("num")
        required_rust = entry.get("rust_version")
        if entry.get("yanked") is not False or not isinstance(version, str) or not isinstance(required_rust, str):
            continue
        try:
            parts = version_parts(version)
            minimum = version_parts(required_rust)
        except ValueError:
            continue
        if parts > current and minimum <= rust:
            compatible.append((parts, version))
    return max(compatible)[1] if compatible else None


def update(root: Path, candidate: str) -> None:
    """
    Rewrite every workflow cargo-edit pin to a newer version.

    :param root: the project root
    :param candidate: the version to pin
    :raises ValueError: if the version is not newer or a workflow does not hold exactly one pin
    """
    current = current_version(root)
    if version_parts(candidate) <= version_parts(current):
        msg = f"cargo-edit {candidate} is not newer than {current}"
        raise ValueError(msg)
    changes = {
        root / ".github/workflows/prepare-release.yaml": (
            f"          crate: cargo-edit\n          version: {current}\n",
            f"          crate: cargo-edit\n          version: {candidate}\n",
        ),
        root / ".github/workflows/_build.yaml": (
            f"          crate: cargo-edit\n          version: {current}\n",
            f"          crate: cargo-edit\n          version: {candidate}\n",
        ),
        root / ".github/workflows/update_dependencies.yaml": (
            f"cargo install cargo-edit --version {current} --locked",
            f"cargo install cargo-edit --version {candidate} --locked",
        ),
    }
    updated = {}
    for path, (old, new) in changes.items():
        content = path.read_text(encoding="utf-8")
        if content.count(old) != 1:
            msg = f"expected one cargo-edit pin in {path}"
            raise ValueError(msg)
        updated[path] = content.replace(old, new)
    for path, content in updated.items():
        path.write_text(content, encoding="utf-8")


def current_version(root: Path) -> str:
    """
    Read the cargo-edit version the workflows pin.

    :param root: the project root
    :returns: the pinned version
    :raises ValueError: if a workflow does not hold exactly one pin, the pins disagree or the version is invalid
    """
    marker = "          crate: cargo-edit\n          version: "
    values = []
    for name in ("prepare-release.yaml", "_build.yaml"):
        path = root / ".github/workflows" / name
        content = path.read_text(encoding="utf-8")
        if content.count(marker) != 1:
            msg = f"expected one cargo-edit pin in {path}"
            raise ValueError(msg)
        values.append(content.split(marker, 1)[1].split("\n", 1)[0])
    install = (root / ".github/workflows/update_dependencies.yaml").read_text(encoding="utf-8")
    if install.count(f"cargo install cargo-edit --version {values[0]} --locked") != 1 or values[0] != values[1]:
        msg = "cargo-edit workflow pins disagree"
        raise ValueError(msg)
    version_parts(values[0])
    return values[0]


def version_parts(value: str) -> tuple[int, int, int]:
    """
    Split a version into a comparable triple, padding a missing patch with zero.

    :param value: a ``major.minor`` or ``major.minor.patch`` version
    :returns: the major, minor and patch numbers
    :raises ValueError: if the version has another shape or a non-numeric part
    """
    parts = value.split(".")
    if len(parts) not in {2, 3} or any(not part.isdecimal() for part in parts):
        msg = f"invalid version: {value}"
        raise ValueError(msg)
    parts.extend(["0"] * (3 - len(parts)))
    return int(parts[0]), int(parts[1]), int(parts[2])


if __name__ == "__main__":
    main()


__all__ = [
    "candidate",
    "current_version",
    "main",
    "update",
    "version_parts",
]
