from __future__ import annotations

import json
import tomllib
from pathlib import Path
from typing import Final, TypeAlias

import jsonschema
import pytest

JsonValue: TypeAlias = "str | int | float | bool | list[JsonValue] | dict[str, JsonValue] | None"
Table: TypeAlias = "dict[str, JsonValue]"

_ROOT: Final[Path] = Path(__file__).resolve().parents[2]
_SCHEMA: Final[Table] = json.loads((_ROOT / "simplint.schema.json").read_text(encoding="utf-8"))
_VALIDATOR: Final[jsonschema.Draft7Validator] = jsonschema.Draft7Validator(_SCHEMA)


def _example_sources() -> list[str]:
    rules = json.loads((_ROOT / "docs" / "_data" / "rules.json").read_text(encoding="utf-8"))
    examples = [example for rule in rules for example in (rule["examples"] or {}).get("parsed", [])]
    return [example["config"]["source"] for example in examples if example.get("config")]


def _simplint_tables(sources: list[str]) -> list[Table]:
    tables = (tomllib.loads(source).get("tool", {}).get("simplint") for source in sources)
    return [table for table in tables if table is not None]


def test_schema_is_a_valid_draft_7_schema() -> None:
    jsonschema.Draft7Validator.check_schema(_SCHEMA)


def test_repo_pyproject_table_is_valid() -> None:
    pyproject = tomllib.loads((_ROOT / "pyproject.toml").read_text(encoding="utf-8"))

    _VALIDATOR.validate(pyproject.get("tool", {}).get("simplint", {}))


@pytest.mark.parametrize(
    "table",
    _simplint_tables(_example_sources()),
    ids=lambda table: json.dumps(table, sort_keys=True),
)
def test_documented_example_is_valid(table: Table) -> None:
    _VALIDATOR.validate(table)


@pytest.mark.parametrize(
    "table",
    [
        pytest.param({"no-such-option": True}, id="unknown_key"),
        pytest.param({"max-paragraph-words": "many"}, id="wrong_type"),
        pytest.param({"select": ["E501"]}, id="ruff_selector"),
        pytest.param({"select": ["SMP9999"]}, id="unknown_code"),
        pytest.param({"output-format": "xml"}, id="unknown_output_format"),
        pytest.param({"naming": {"keep-name": []}}, id="unknown_nested_key"),
    ],
)
def test_invalid_table_is_rejected(table: Table) -> None:
    with pytest.raises(jsonschema.ValidationError):
        _VALIDATOR.validate(table)
