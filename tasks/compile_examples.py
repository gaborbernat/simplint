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


@dataclass(frozen=True, slots=True, kw_only=True)
class Interpreter:
    # A CPython of a minor version, found by uv.
    minor: int
    path: str


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
    failures = check(rule_blocks(options.simplint), [resolve(minor) for minor in options.versions])
    for failure in failures:
        print(failure)
    return int(bool(failures))


def minor_version(text: str) -> int:
    # Read a CPython version such as 3.12 as its minor number.
    #
    # :param text: the version, of the form 3.X
    # :returns: the minor number
    # :raises argparse.ArgumentTypeError: if the text is not a 3.X version
    if match := re.fullmatch(r"3\.(\d+)", text):
        return int(match.group(1))
    msg = f"expected 3.X, got {text!r}"
    raise argparse.ArgumentTypeError(msg)


def resolve(minor: int) -> Interpreter:
    # Find the CPython of a minor version with uv, which the CI job installs ahead of this task.
    #
    # The project's own requires-python is skipped, as it would warn about the older versions this task exists to test.
    #
    # :param minor: the minor number
    # :returns: the interpreter
    # :raises ValueError: if uv finds no interpreter of that version
    found = subprocess.run(
        ["uv", "python", "find", "--no-project", f"3.{minor}"], capture_output=True, text=True, check=False
    )
    if found.returncode:
        msg = f"no CPython 3.{minor} for the examples: {found.stderr.strip()}"
        raise ValueError(msg)
    return Interpreter(minor=minor, path=found.stdout.strip())


def rule_blocks(simplint: str) -> list[Block]:
    # Collect the example blocks of every rule page, each with the floor its group's config gives it.
    #
    # :param simplint: the simplint executable
    # :returns: the blocks, in rule order
    listing = subprocess.run(
        [simplint, "rules", "--output-format", "json"], capture_output=True, text=True, check=True
    ).stdout
    return [
        Block(
            name=f"{rule['code']}:{block['line']}:{block['role']}",
            source=block["source"],
            floor=block_floor(group["config"]),
        )
        for rule in json.loads(listing)
        for group in rule["examples"]["parsed"]
        for block in group["blocks"]
    ]


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


def check(blocks: list[Block], interpreters: list[Interpreter]) -> list[str]:
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
    "Block",
    "Interpreter",
    "block_floor",
    "check",
    "main",
    "minor_version",
    "resolve",
    "rule_blocks",
]
