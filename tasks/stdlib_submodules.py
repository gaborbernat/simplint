# /// script
# requires-python = ">=3.12"
# ///
"""Generate the standard-library submodule table for dependency-import-guard from CPython source trees."""

from __future__ import annotations

import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from types import MappingProxyType
from typing import Final

_ROOT: Final[Path] = Path(__file__).resolve().parents[1]
_TABLE: Final[Path] = _ROOT / "src" / "rules" / "imports" / "stdlib_submodules.rs"
# Read tagged releases or a maintenance branch when tracking changes to its standard library.
_REFS: Final[MappingProxyType[int, str]] = MappingProxyType({
    10: "v3.10.0",
    11: "v3.11.0",
    12: "v3.12.0",
    13: "v3.13.0",
    14: "v3.14.0",
    15: "origin/3.15",
})
_LATEST: Final[int] = max(_REFS)
# os.py registers os.path in sys.modules rather than shipping a file of that name, so the file listing never shows it.
_REGISTERED: Final[frozenset[str]] = frozenset({"os.path"})


@dataclass(frozen=True, slots=True, kw_only=True)
class Span:
    # Endpoints of the last consecutive run of releases. The upper bound is exclusive and None means still present.
    added: int
    removed: int | None


def main(cpython: Path) -> None:
    presence = {minor: dotted_names(listed(cpython, ref)) | _REGISTERED for minor, ref in _REFS.items()}
    _TABLE.write_text(render(spans(presence)), encoding="utf-8")
    subprocess.check_call(["rustfmt", "--edition", "2024", str(_TABLE)])


def listed(cpython: Path, ref: str) -> list[str]:
    listing = subprocess.check_output(
        ["git", "-C", str(cpython), "ls-tree", "-r", "--name-only", ref, "Lib/"], text=True
    )
    return listing.splitlines()


def dotted_names(paths: list[str]) -> set[str]:
    # Turn the Python module files under Lib/ into the dotted names of the submodules they hold. Top-level modules
    # stay with Ruff's table, which lists them. Test trees and dunder helper packages such as __phello__ are not what
    # a project guards an import of.
    excluded = frozenset({"test", "tests", "idlelib", "turtledemo"})
    names: set[str] = set()
    for path in paths:
        if not path.endswith(".py"):
            continue
        parts = path.removeprefix("Lib/").removesuffix(".py").split("/")
        if parts[-1] == "__init__":
            parts.pop()
        if len(parts) <= 1 or excluded.intersection(parts) or any(part.startswith("__") for part in parts):
            continue
        names.add(".".join(parts))
    return names


def spans(presence: dict[int, set[str]]) -> dict[str, Span]:
    # If a submodule disappeared and returned, a project targeting a version before the gap cannot rely on it.
    # Use the start of the most recent consecutive run.
    minors: dict[str, list[int]] = {}
    for minor in sorted(presence):
        for name in presence[minor]:
            minors.setdefault(name, []).append(minor)
    found: dict[str, Span] = {}
    for name, present in minors.items():
        start = present[-1]
        while start - 1 in present:
            start -= 1
        last = present[-1]
        found[name] = Span(added=start, removed=None if last == _LATEST else last + 1)
    return found


def render(table: dict[str, Span]) -> str:
    # The Rust lookup uses binary search, so emit the entries in name order.
    rows = "".join(
        f'    Submodule {{ name: "{name}", added: {span.added}, '
        f"removed: {'None' if span.removed is None else f'Some({span.removed})'} }},\n"
        for name, span in sorted(table.items())
    )
    return (
        "// Generated from CPython's Lib/ trees by tasks/stdlib_submodules.py.\n"
        "\n"
        "pub(super) struct Submodule {\n"
        "    pub(super) name: &'static str,\n"
        "    pub(super) added: u8,\n"
        "    pub(super) removed: Option<u8>,\n"
        "}\n"
        "\n"
        f"pub(super) const LATEST_MINOR: u8 = {_LATEST};\n"
        "\n"
        "// For each listed submodule, keep its last consecutive run in the CPython releases sampled from 3.10.\n"
        "// added is inclusive. removed is exclusive, or None if the latest sampled release still has the submodule.\n"
        "pub(super) const STDLIB_SUBMODULES: &[Submodule] = &[\n"
        f"{rows}"
        "];\n"
    )


if __name__ == "__main__":
    main(Path(sys.argv[1]))


__all__ = [
    "Span",
    "dotted_names",
    "listed",
    "main",
    "render",
    "spans",
]
