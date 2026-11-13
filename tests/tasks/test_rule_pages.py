from __future__ import annotations

import importlib
from typing import TYPE_CHECKING

import pytest

from docs.rules_data import parse

if TYPE_CHECKING:
    from pathlib import Path
    from types import ModuleType


@pytest.fixture
def rule_pages(monkeypatch: pytest.MonkeyPatch, root: Path) -> ModuleType:
    # the page renderer is a build module that imports the rule list beside it, the way Sphinx runs it
    monkeypatch.syspath_prepend(str(root / "docs"))
    return importlib.import_module("_ext.rule_pages")


def test_rule_page_labels_a_block_with_its_file_and_attributes(rule_pages: ModuleType) -> None:
    explanation = """\
Flags it.

## Example

```python report file=tasks.py executable
X = 1
```

## What it does

### Active on

- **Selected**: a case.

  ```python report select=SMP101
  Y = 2
  ```

  ```python fixed
  Y = 3
  ```
"""
    page = rule_pages._rule_page(  # ruff: ignore[private-member-access] # the build's own renderer
        {
            "code": "SMP106",
            "name": "unguarded-entrypoint-call",
            "summary": "A trailing top-level call",
            "fix": "None",
            "explanation": explanation,
            "examples": {"parsed": parse(explanation)},
            "family": "naming",
            "group": "Naming",
            "short_group": "Naming",
            "cross_file": False,
            "opt_in": False,
        }
    )

    assert (
        "`tasks.py` (executable)\n\n```{code-block} python" in page,
        "Before, with SMP101 selected" in page,
        "After" in page,
        "badge opt-in" in page,
    ) == (True, True, True, False)


def test_rule_page_badges_an_opt_in_rule(rule_pages: ModuleType) -> None:
    page = rule_pages._rule_page(  # ruff: ignore[private-member-access] # the build's own renderer
        {
            "code": "SMP407",
            "name": "init-module-content",
            "summary": "`__init__.py` holding code beyond its API",
            "fix": "None",
            "explanation": "Flags it.\n\n## Why is this bad?\n\nIt runs first.\n",
            "examples": {"parsed": []},
            "family": "api",
            "group": "Public API",
            "short_group": "Public API",
            "cross_file": False,
            "opt_in": True,
        }
    )

    assert '<a class="badge opt-in" href="../how-to/select-rules.html#opt-in-rules"' in page
