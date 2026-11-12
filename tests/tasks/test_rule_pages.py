from __future__ import annotations

import importlib
from typing import TYPE_CHECKING

import pytest

from docs.rules_data import parse

if TYPE_CHECKING:
    from pathlib import Path
    from types import ModuleType

    from docs.rules_data import RuleEntry

EXPLANATION = """\
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


@pytest.fixture
def rule_pages(monkeypatch: pytest.MonkeyPatch, root: Path) -> ModuleType:
    # the page renderer is a build module that imports the rule list beside it, the way Sphinx runs it
    monkeypatch.syspath_prepend(str(root / "docs"))
    return importlib.import_module("_ext.rule_pages")


def test_rule_page_labels_a_block_with_its_file_and_attributes(rule_pages: ModuleType) -> None:
    page = rule_pages._rule_page(_entry(EXPLANATION))  # ruff: ignore[private-member-access] # the build's own renderer

    assert (
        "`tasks.py` (executable)\n\n```{code-block} python" in page,
        "Before, with SMP101 selected" in page,
        "After" in page,
    ) == (True, True, True)


def _entry(explanation: str) -> RuleEntry:
    return {
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
    }
