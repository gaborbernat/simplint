"""Compile the examples on the rule pages under each Python release their block must support."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tomllib
from dataclasses import dataclass
from typing import Final

from tasks.interpreters import Interpreter, minor_version, resolve

#: The oldest release a block compiles under when its group names no lower bound.
_DEFAULT_FLOOR: Final[int] = 11
#: Reads the sources as a JSON list and prints, for each, its syntax error or null. Warnings are dropped: some examples
#: show invalid code on purpose, and a syntax warning is not a failure.
_COMPILE: Final[str] = """\
import json
import sys
import warnings

warnings.simplefilter("ignore")
errors = []
for source in json.load(sys.stdin):
    try:
        compile(source, "<example>", "exec", dont_inherit=True)
    except SyntaxError as exc:
        errors.append(f"{exc.msg} (line {exc.lineno})")
    else:
        errors.append(None)
json.dump(errors, sys.stdout)
"""


@dataclass(frozen=True, slots=True, kw_only=True)
class Block:
    # An example block of a rule page, and the oldest minor release it must compile under.
    name: str
    source: str
    floor: int


class _Options(argparse.Namespace):
    simplint: str
    versions: list[int]


def main(arguments: list[str]) -> int:
    # Compile every rule-page example under each interpreter and report the blocks that fail where they must compile.
    #
    # :param arguments: the command line, without the program name
    # :returns: the exit status, 1 when a block fails under an interpreter at or above its floor
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--simplint", default="simplint", help="the simplint executable whose rule pages are compiled")
    parser.add_argument(
        "--python",
        dest="versions",
        action="append",
        required=True,
        type=minor_version,
        metavar="3.X",
        help="a CPython minor version to compile under, found with uv; repeat for each",
    )
    options = parser.parse_args(arguments, namespace=_Options())
    listing = subprocess.run(
        [options.simplint, "rules", "--output-format", "json"], capture_output=True, text=True, check=True
    ).stdout
    blocks = [
        Block(
            name=f"{rule['code']}:{block['line']}:{block['role']}",
            source=block["source"],
            floor=block_floor(group["config"]),
        )
        for rule in json.loads(listing)
        for group in rule["examples"]["parsed"]
        for block in group["blocks"]
        # a page shows documentation files, such as a reStructuredText page a rule reads, next to its Python
        if block["language"] == "python"
    ]
    failures = _check(blocks, [resolve(minor) for minor in options.versions])
    for failure in failures:
        print(failure)
    return int(bool(failures))


def block_floor(config: dict[str, str] | None) -> int:
    # Read the oldest minor release a group supports from its config: the target version, else the lower bound.
    #
    # :param config: the group's config, as its ``source`` text, or None when it has none
    # :returns: the minor number, the default when the config names neither
    if config is None:
        return _DEFAULT_FLOOR
    settings = tomllib.loads(config["source"])
    if target := settings.get("tool", {}).get("ruff", {}).get("target-version"):
        return int(target.removeprefix("py3"))
    if match := re.match(r">=\s*3\.(\d+)", settings.get("project", {}).get("requires-python", "")):
        return int(match.group(1))
    return _DEFAULT_FLOOR


def _check(blocks: list[Block], interpreters: list[Interpreter]) -> list[str]:
    # Compile each block under the interpreters at or above its floor, one subprocess per interpreter.
    #
    # :param blocks: the example blocks
    # :param interpreters: the CPythons to compile under
    # :returns: one line per failure, naming the block and the release that rejects it
    failures = []
    for interpreter in interpreters:
        required = [block for block in blocks if interpreter.minor >= block.floor]
        errors = subprocess.run(
            [interpreter.path, "-I", "-c", _COMPILE],
            input=json.dumps([block.source for block in required]),
            capture_output=True,
            text=True,
            check=True,
        ).stdout
        failures += [
            f"{block.name} py3.{interpreter.minor}: {error}"
            for block, error in zip(required, json.loads(errors), strict=True)
            if error
        ]
    return failures


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))


__all__ = [
    "block_floor",
    "main",
]
