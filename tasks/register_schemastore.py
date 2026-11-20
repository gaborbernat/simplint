"""Register simplint's JSON Schema in a SchemaStore checkout as a partial that pyproject.json references."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import TYPE_CHECKING, Final

if TYPE_CHECKING:
    from collections.abc import Sequence

_TOOL_ANCHOR: Final[re.Pattern[str]] = re.compile(r'(?P<indent> *)"taskipy": \{')
_MISSING_ANCHOR: Final[re.Pattern[str]] = re.compile(
    r'(?P<indent> *)"partial-tox\.json", // classic pyproject\.json\[tool\.tox\]'
)
_EXTERNAL_ANCHOR: Final[re.Pattern[str]] = re.compile(r'(?P<indent> *)"partial-taskipy\.json",')


def main(argv: Sequence[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description="Register simplint's JSON Schema with a SchemaStore checkout")
    parser.add_argument("schema", type=Path, help="simplint's JSON Schema")
    parser.add_argument("store", type=Path, help="root of the SchemaStore checkout")
    parser.add_argument("options", type=Path, help="sample [tool.simplint] options for the checkout's tests")
    args = parser.parse_args(argv)
    register(args.schema, args.store, args.options)


def register(schema: Path, store: Path, options: Path) -> None:
    # Validate insertion points before writing so a missing anchor leaves the checkout intact.
    # Existing references are reused. The schema and sample options are refreshed on each run.
    pyproject = store / "src" / "schemas" / "json" / "pyproject.json"
    pyproject_text = pyproject.read_text(encoding="utf-8")
    partial_name = "partial-simplint.json"
    if partial_name not in pyproject_text:
        tool_entry = (
            '{indent}"simplint": {{\n'
            '{indent}  "$ref": "partial-simplint.json",\n'
            '{indent}  "title": "Linter",\n'
            '{indent}  "description": "Ruff companion linter with extra checks that make Python easier to read and '
            'review."\n'
            "{indent}}},\n"
        )
        pyproject_text = _insert_before(pyproject_text, _TOOL_ANCHOR, tool_entry, pyproject.name)
    validation = store / "src" / "schema-validation.jsonc"
    validation_text = validation.read_text(encoding="utf-8")
    if partial_name not in validation_text:
        missing_entry = '{indent}"partial-simplint.json", // pyproject.json[tool.simplint]\n'
        validation_text = _insert_before(validation_text, _MISSING_ANCHOR, missing_entry, validation.name)
        external_entry = '{indent}"partial-simplint.json",\n'
        validation_text = _insert_before(validation_text, _EXTERNAL_ANCHOR, external_entry, validation.name)

    definition = json.loads(schema.read_text(encoding="utf-8"))
    definition["$id"] = f"https://json.schemastore.org/{partial_name}"
    (store / "src" / "schemas" / "json" / partial_name).write_text(
        f"{json.dumps(definition, indent=2)}\n", encoding="utf-8"
    )
    pyproject.write_text(pyproject_text, encoding="utf-8")
    validation.write_text(validation_text, encoding="utf-8")
    test_options = store / "src" / "test" / "partial-simplint" / "options.json"
    test_options.parent.mkdir(exist_ok=True)
    test_options.write_text(options.read_text(encoding="utf-8"), encoding="utf-8")


def _insert_before(text: str, anchor: re.Pattern[str], entry: str, name: str) -> str:
    # Insert the entry above each line the anchor matches. The entry's ``{indent}`` takes the anchor line's indentation,
    # so the file keeps its layout without a JSON round trip that would reflow it.
    lines: list[str] = []
    matches = 0
    for line in text.splitlines(keepends=True):
        if (match := anchor.fullmatch(line.rstrip("\n"))) is not None:
            matches += 1
            lines.append(entry.format(indent=match["indent"]))
        lines.append(line)
    if matches == 0:
        msg = f"{name} has no line matching {anchor.pattern!r}"
        raise ValueError(msg)
    return "".join(lines)


if __name__ == "__main__":
    main()


__all__ = [
    "main",
    "register",
]
