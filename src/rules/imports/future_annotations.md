## What it does

Checks for `from __future__ import annotations` ([PEP 563](https://peps.python.org/pep-0563/)) at module level when the
target version is Python 3.14 or newer. simplint reads the target version from Ruff's configuration, as `ruff check`
does: [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) or
[`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), then
`project.requires-python`, then Ruff's default.

## Why is this bad?

Since Python 3.14, annotations are
[evaluated lazily](https://docs.python.org/3/whatsnew/3.14.html#whatsnew314-deferred-annotations) by default
([PEP 649](https://peps.python.org/pep-0649/), [PEP 749](https://peps.python.org/pep-0749/)). The import no longer
avoids forward-reference errors or the cost of evaluating annotations at import time. Its only remaining effect is to
turn annotations into strings for code that inspects them at runtime.

## Example

```python
from __future__ import annotations, generator_stop
```

Use instead:

```python
from __future__ import generator_stop
```

## Fix safety

The fix removes `annotations` from the import, or deletes the statement when `annotations` is its only name. If that
statement is the only one in the module, the fix replaces it with `pass`. No fix is offered when a statement that
imports only `annotations` shares its line with other code.

After the fix, code that reads `__annotations__` and expects strings gets evaluated objects
([annotations best practices](https://docs.python.org/3/howto/annotations.html)). The fix is unsafe when the module
refers to the name `TYPE_CHECKING` in code; a mention in a comment or string does not count. Lazy annotations are still
evaluated on access, and an annotation that names something imported only under
[`if TYPE_CHECKING:`](https://docs.python.org/3/library/typing.html#typing.TYPE_CHECKING) raises `NameError` there.
Otherwise the fix is safe.

## Options

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python)

## Conflicts with Ruff

When Ruff's isort setting [`required-imports`](https://docs.astral.sh/ruff/settings/#lint_isort_required-imports) lists
`from __future__ import annotations`,
[missing-required-import (I002)](https://docs.astral.sh/ruff/rules/missing-required-import/) asks for the import this
rule removes. simplint then withdraws the fix and keeps the report; drop the import from `required-imports` once the
project targets Python 3.14.
