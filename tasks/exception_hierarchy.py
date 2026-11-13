# /// script
# requires-python = ">=3.10"
# ///
"""Rewrite the table of standard exception classes that the exception rules read, from the oldest and newest Python."""

from __future__ import annotations

import importlib
import inspect
import json
import subprocess
import sys
from pathlib import Path
from typing import Final, TypedDict

_ROOT: Final[Path] = Path(__file__).resolve().parents[1]
_TABLE: Final[Path] = _ROOT / "src" / "rules" / "exception_classes" / "hierarchy.rs"
# The standard library modules whose public exception classes a project commonly raises or catches.
_MODULES: Final[tuple[str, ...]] = (
    "argparse",
    "asyncio",
    "binascii",
    "calendar",
    "concurrent.futures",
    "configparser",
    "copy",
    "csv",
    "decimal",
    "email.errors",
    "ftplib",
    "getopt",
    "graphlib",
    "gzip",
    "http.client",
    "http.cookiejar",
    "importlib.metadata",
    "io",
    "json",
    "locale",
    "lzma",
    "mailbox",
    "multiprocessing",
    "netrc",
    "os",
    "pickle",
    "plistlib",
    "poplib",
    "queue",
    "re",
    "select",
    "shutil",
    "smtplib",
    "socket",
    "sqlite3",
    "ssl",
    "statistics",
    "struct",
    "subprocess",
    "tarfile",
    "tempfile",
    "threading",
    "tomllib",
    "unittest",
    "urllib.error",
    "webbrowser",
    "xml.etree.ElementTree",
    "xml.parsers.expat",
    "zipfile",
    "zlib",
    "zoneinfo",
)


class Entry(TypedDict, total=False):
    # A name the table holds: the direct bases of the class it defines, or the class it gives a second name to.
    bases: list[str]
    alias: str


def main(interpreters: list[str]) -> None:
    # Walk the exception classes under each interpreter and rewrite the table with the names all of them agree on.
    #
    # :param interpreters: the Python executables to walk, the oldest and the newest supported version
    walks = [
        json.loads(subprocess.check_output([interpreter, "-I", __file__, "--walk"])) for interpreter in interpreters
    ]
    _TABLE.write_text(render(agreed(walks)), encoding="utf-8")
    subprocess.check_call(["rustfmt", "--edition", "2024", str(_TABLE)])


def walk(modules: tuple[str, ...]) -> dict[str, Entry]:
    # Collect the builtin exceptions and the public exception classes of each module, with every class they derive from.
    # The module that defines a class is walked too, since it can give a name to a class another version defines
    # apart, as `asyncio.exceptions.TimeoutError` names the builtin `TimeoutError` from 3.11 on.
    #
    # :param modules: the dotted names of the modules to import
    # :returns: each name the table holds, keyed by its dotted form
    entries: dict[str, Entry] = {}
    classes: list[type] = []
    pending = ["builtins", *modules]
    walked: set[str] = set()
    while pending:
        if (module := pending.pop()) in walked:
            continue
        walked.add(module)
        for name, value in _exceptions(module):
            classes.append(value)
            pending.append(value.__module__)
            if (alias := name if module == "builtins" else f"{module}.{name}") != dotted(value):
                entries[alias] = {"alias": dotted(value)}
    while classes:
        cls = classes.pop()
        if "bases" not in entries.get(name := dotted(cls), {}):
            parents = [base for base in cls.__bases__ if base is not object]
            entries[name] = {"bases": [dotted(base) for base in parents]}
            classes.extend(parents)
    return entries


def _exceptions(module: str) -> list[tuple[str, type]]:
    # The public exception classes a module holds, none for a module this interpreter lacks, as `tomllib` before 3.11.
    #
    # :param module: the dotted name of the module
    # :returns: each class with the name the module gives it
    try:
        imported = importlib.import_module(module)
    except ModuleNotFoundError:
        return []
    return [
        (name, value)
        for name, value in inspect.getmembers(imported, inspect.isclass)
        if not name.startswith("_") and issubclass(value, BaseException)
    ]


def dotted(cls: type) -> str:
    # The name a class is defined under: bare for a builtin, else its module and qualified name.
    #
    # :param cls: the class
    # :returns: the dotted name
    return cls.__qualname__ if cls.__module__ == "builtins" else f"{cls.__module__}.{cls.__qualname__}"


def agreed(walks: list[dict[str, Entry]]) -> dict[str, Entry]:
    # Keep each name every walk that has it agrees on, so the table holds on every version a project targets. A name a
    # version lacks, as one added later, still holds where it exists.
    #
    # :param walks: the names of each interpreter
    # :returns: the names no two walks disagree on
    merged: dict[str, Entry] = {}
    conflicting: set[str] = set()
    for entries in walks:
        for name, value in entries.items():
            if merged.setdefault(name, value) != value:
                conflicting.add(name)
    return {name: value for name, value in merged.items() if name not in conflicting}


def render(entries: dict[str, Entry]) -> str:
    # Write the Rust tables, each sorted by name so a lookup can binary search it. An alias whose class has no entry
    # leads nowhere, so it is left out.
    #
    # :param entries: the names, each holding the ``bases`` of its class or the ``alias`` it stands for
    # :returns: the Rust source of the tables
    ordered = sorted(entries.items())
    base_rows = "".join(
        f'    ("{name}", &[{_quoted(entry["bases"])}]),\n' for name, entry in ordered if "bases" in entry
    )
    alias_rows = "".join(
        f'    ("{name}", "{entry["alias"]}"),\n'
        for name, entry in ordered
        if "bases" in entries.get(entry.get("alias", ""), {})
    )
    return (
        "// Generated by tasks/exception_hierarchy.py from the oldest and newest supported Python; regenerate instead\n"
        "// of editing. A class whose bases differ between the two is left out, so the table holds on every version.\n"
        "\n"
        "// Each class, sorted by name, with its direct bases.\n"
        "pub(super) const BASES: &[(&str, &[&str])] = &[\n"
        f"{base_rows}"
        "];\n"
        "\n"
        "// The second names classes of `BASES` go by, sorted by name, such as `IOError` for `OSError`.\n"
        "pub(super) const ALIASES: &[(&str, &str)] = &[\n"
        f"{alias_rows}"
        "];\n"
    )


def _quoted(names: list[str]) -> str:
    # Join names as Rust string literals.
    #
    # :param names: the names
    # :returns: the comma-separated literals
    return ", ".join(f'"{name}"' for name in names)


if __name__ == "__main__":
    if sys.argv[1:] == ["--walk"]:
        print(json.dumps(walk(_MODULES)))
    else:
        main(sys.argv[1:])


__all__ = [
    "Entry",
    "agreed",
    "dotted",
    "main",
    "render",
    "walk",
]
