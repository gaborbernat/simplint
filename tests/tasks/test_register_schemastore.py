from __future__ import annotations

import json
from pathlib import Path
from typing import Final

import jsonschema
import pytest

from tasks.register_schemastore import main, register

_ROOT: Final[Path] = Path(__file__).resolve().parents[2]
_SCHEMA: Final[Path] = _ROOT / "simplint.schema.json"
_OPTIONS: Final[Path] = _ROOT / "tasks" / "schemastore_options.json"

# Excerpts of SchemaStore's files around the entries the registration adds next to.
_PYPROJECT: Final[str] = """\
{
  "properties": {
    "tool": {
      "properties": {
        "repo-review": {
          "$ref": "partial-repo-review.json",
          "title": "Repository Reviewer",
          "description": "Review a repository for best practices."
        },
        "taskipy": {
          "$ref": "partial-taskipy.json",
          "title": "Task Runner",
          "description": "The complementary task runner for python."
        },
        "tombi": {
          "$ref": "https://www.schemastore.org/tombi.json"
        }
      }
    }
  }
}
"""
_VALIDATION: Final[str] = """\
{
  "missingCatalogUrl": [
    // Below this line are subschemas that are included from other schema
    "partial-scheduled.json", // pyproject.json[tool.scheduled]
    "partial-tox.json", // classic pyproject.json[tool.tox]
    "tox.json",
  ],
  "options": {
    "pep-723.json": {
      "externalSchema": [
        "partial-mypy.json",
        "partial-taskipy.json",
        "tox.json"
      ]
    },
    "pyproject.json": {
      "externalSchema": [
        "partial-setuptools-scm.json",
        "partial-taskipy.json",
        "partial-tox.json"
      ]
    }
  }
}
"""


@pytest.fixture
def store(tmp_path: Path) -> Path:
    root = tmp_path / "schemastore"
    (root / "src" / "schemas" / "json").mkdir(parents=True)
    (root / "src" / "test").mkdir()
    (root / "src" / "schemas" / "json" / "pyproject.json").write_text(_PYPROJECT, encoding="utf-8")
    (root / "src" / "schema-validation.jsonc").write_text(_VALIDATION, encoding="utf-8")
    return root


def _register(store: Path) -> None:
    register(_SCHEMA, store, _OPTIONS)


def test_register_writes_partial_with_schemastore_id(store: Path) -> None:
    _register(store)

    partial = json.loads((store / "src" / "schemas" / "json" / "partial-simplint.json").read_text(encoding="utf-8"))
    expected = json.loads(_SCHEMA.read_text(encoding="utf-8"))
    expected["$id"] = "https://json.schemastore.org/partial-simplint.json"
    assert partial == expected


def test_register_references_partial_from_pyproject(store: Path) -> None:
    _register(store)
    registered = """\
{
  "properties": {
    "tool": {
      "properties": {
        "repo-review": {
          "$ref": "partial-repo-review.json",
          "title": "Repository Reviewer",
          "description": "Review a repository for best practices."
        },
        "simplint": {
          "$ref": "partial-simplint.json",
          "title": "Linter",
          "description": "Ruff companion linter with extra checks that make Python easier to read and review."
        },
        "taskipy": {
          "$ref": "partial-taskipy.json",
          "title": "Task Runner",
          "description": "The complementary task runner for python."
        },
        "tombi": {
          "$ref": "https://www.schemastore.org/tombi.json"
        }
      }
    }
  }
}
"""

    assert (store / "src" / "schemas" / "json" / "pyproject.json").read_text(encoding="utf-8") == registered


def test_register_lists_partial_in_catalog_and_external_schemas(store: Path) -> None:
    _register(store)
    registered = """\
{
  "missingCatalogUrl": [
    // Below this line are subschemas that are included from other schema
    "partial-scheduled.json", // pyproject.json[tool.scheduled]
    "partial-simplint.json", // pyproject.json[tool.simplint]
    "partial-tox.json", // classic pyproject.json[tool.tox]
    "tox.json",
  ],
  "options": {
    "pep-723.json": {
      "externalSchema": [
        "partial-mypy.json",
        "partial-simplint.json",
        "partial-taskipy.json",
        "tox.json"
      ]
    },
    "pyproject.json": {
      "externalSchema": [
        "partial-setuptools-scm.json",
        "partial-simplint.json",
        "partial-taskipy.json",
        "partial-tox.json"
      ]
    }
  }
}
"""

    assert (store / "src" / "schema-validation.jsonc").read_text(encoding="utf-8") == registered


def test_register_copies_options(store: Path) -> None:
    _register(store)

    copied = store / "src" / "test" / "partial-simplint" / "options.json"
    assert copied.read_text(encoding="utf-8") == _OPTIONS.read_text(encoding="utf-8")


def test_register_is_idempotent(store: Path) -> None:
    _register(store)
    first = {path: path.read_text(encoding="utf-8") for path in store.rglob("*") if path.is_file()}

    _register(store)

    assert {path: path.read_text(encoding="utf-8") for path in store.rglob("*") if path.is_file()} == first


@pytest.mark.parametrize(
    ("relative", "broken"),
    [
        pytest.param(
            "src/schemas/json/pyproject.json",
            _PYPROJECT.replace('"taskipy": {', '"task": {'),
            id="no_taskipy_tool",
        ),
        pytest.param(
            "src/schema-validation.jsonc",
            _VALIDATION.replace('"partial-tox.json", // classic pyproject.json[tool.tox]', '"tox.json",'),
            id="no_tox_catalog_entry",
        ),
        pytest.param(
            "src/schema-validation.jsonc",
            _VALIDATION.replace('"partial-taskipy.json",', '"tox.json",'),
            id="no_taskipy_external_schema",
        ),
    ],
)
def test_register_fails_without_anchor_and_writes_nothing(store: Path, relative: str, broken: str) -> None:
    (store / relative).write_text(broken, encoding="utf-8")
    before = {path: path.read_text(encoding="utf-8") for path in store.rglob("*") if path.is_file()}

    with pytest.raises(ValueError, match="has no line matching"):
        _register(store)

    assert {path: path.read_text(encoding="utf-8") for path in store.rglob("*") if path.is_file()} == before


def test_main_registers_from_arguments(store: Path) -> None:
    main([str(_SCHEMA), str(store), str(_OPTIONS)])

    assert (store / "src" / "schemas" / "json" / "partial-simplint.json").is_file()


def test_committed_options_are_valid() -> None:
    schema = json.loads(_SCHEMA.read_text(encoding="utf-8"))
    jsonschema.Draft7Validator(schema).validate(json.loads(_OPTIONS.read_text(encoding="utf-8")))
