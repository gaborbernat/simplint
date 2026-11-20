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
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--simplint", default="simplint", help="the simplint executable whose rule pages are compiled")
    parser.add_argument(
        "--python",
        dest="versions",
        action="append",
        required=True,
        type=minor_version,
        metavar="3.X",
        help="CPython version to locate with uv (repeat for each version)",
    )
    options = parser.parse_args(arguments, namespace=_Options())
    listing = subprocess.run(
        [options.simplint, "rules", "--output-format", "json"], capture_output=True, text=True, check=True
    ).stdout
    blocks: list[Block] = []
    for rule in json.loads(listing):
        for group in rule["examples"]["parsed"]:
            blocks.extend(
                Block(
                    name=f"{rule['code']}:{block['line']}:{block['role']}",
                    source=block["source"],
                    floor=block_floor(group["config"]),
                )
                for block in group["blocks"]
                # a page shows documentation files, such as a reStructuredText page a rule reads, next to its Python
                if block["language"] == "python"
            )
    failures = _check(blocks, [resolve(minor) for minor in options.versions])
    for failure in failures:
        print(failure)
    return int(bool(failures))


_BLOCK_FLOOR_PATTERN: Final[re.Pattern[str]] = re.compile(r">=\s*3\.(\d+)")


def block_floor(config: dict[str, str] | None) -> int:
    # Look for Ruff's target version first, then a requires-python value starting with >=3.X.
    default_floor: int = 11
    if config is None:
        return default_floor
    settings = tomllib.loads(config["source"])
    if target := settings.get("tool", {}).get("ruff", {}).get("target-version"):
        return int(target.removeprefix("py3"))
    if match := _BLOCK_FLOOR_PATTERN.match(settings.get("project", {}).get("requires-python", "")):
        return int(match.group(1))
    return default_floor


def _check(blocks: list[Block], interpreters: list[Interpreter]) -> list[str]:
    # Some examples deliberately show code that emits syntax warnings. Only compilation errors fail the check.
    compile_sources = """\
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
    failures = []
    for interpreter in interpreters:
        required = [block for block in blocks if interpreter.minor >= block.floor]
        errors = subprocess.run(
            [interpreter.path, "-I", "-c", compile_sources],
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
