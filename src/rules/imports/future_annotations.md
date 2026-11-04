Flags `from __future__ import annotations` when the target version is Python 3.14 or newer. Annotations are evaluated
lazily there by default, so the import no longer does what it was added for.

## Why is this bad?

Since Python 3.14, annotations are
[evaluated lazily](https://docs.python.org/3/whatsnew/3.14.html#whatsnew314-deferred-annotations) by default
([PEP 649](https://peps.python.org/pep-0649/), [PEP 749](https://peps.python.org/pep-0749/)). The import no longer
avoids forward-reference errors or the cost of evaluating annotations at import time. Its only remaining effect is to
turn annotations into strings for code that inspects them at runtime.

## Example

```toml config
[project]
name = "demo"
requires-python = ">=3.14"
```

```python report
from __future__ import annotations, generator_stop
```

Use instead:

```python ok
from __future__ import generator_stop
```

## Conflicts with Ruff

When Ruff's isort setting [`required-imports`](https://docs.astral.sh/ruff/settings/#lint_isort_required-imports) lists
`from __future__ import annotations`,
[missing-required-import](https://docs.astral.sh/ruff/rules/missing-required-import/) adds back the import this rule
removes. simplint stops with an error before linting while both apply to a file that targets Python 3.14 or newer; drop
the import from `required-imports` once the project targets Python 3.14, or turn this rule off.

## Configuration

- The Python version the code targets, taken from the first of these that applies; see
  [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version):

  1. [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version) in `[tool.ruff]`: for
     the files a pattern matches

     ```toml config
     [tool.ruff.per-file-target-version]
     "new/*.py" = "py314"
     ```

     ```python report file=new/module.py
     from __future__ import annotations

     VALUE = 1
     ```

  1. [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) in `[tool.ruff]`: for every other file

     ```toml config
     [tool.ruff]
     target-version = "py313"
     ```

     ```python ok
     from __future__ import annotations

     VALUE = 1
     ```

  1. [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python):
     its lowest version, when Ruff sets neither

     ```toml config
     [project]
     name = "demo"
     requires-python = ">=3.14"
     ```

     ```python report
     from __future__ import annotations

     VALUE = 1
     ```

## What it does

### Active on

- **Module-level `from __future__ import annotations`** ([PEP 563](https://peps.python.org/pep-0563/)) in a file that
  targets Python 3.14 or newer, where lazy evaluation is already the default. **Unsafe fix** when the module refers to
  the name `TYPE_CHECKING` in code; a mention in a comment or string does not count. Lazy annotations are still
  evaluated on access, and an annotation that names something imported only under
  [`if TYPE_CHECKING:`](https://docs.python.org/3/library/typing.html#typing.TYPE_CHECKING) raises `NameError` there.
  Otherwise the fix is safe.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.14"
  ```

  ```python report
  from __future__ import annotations

  VALUE = 1
  ```

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.14"
  ```

  ```python report
  # Demo module.
  from __future__ import annotations
  from typing import TYPE_CHECKING

  if TYPE_CHECKING:
      from os import PathLike


  def path(value: PathLike) -> None:
      pass
  ```

  ```python fixed
  # Demo module.
  from typing import TYPE_CHECKING

  if TYPE_CHECKING:
      from os import PathLike


  def path(value: PathLike) -> None:
      pass
  ```

### Exceptions

- **Files that target a version before Python 3.14**: there the import still defers evaluation, which annotations do not
  do by default.

  ```toml config
  [tool.ruff]
  target-version = "py313"
  ```

  ```python ok
  from __future__ import annotations

  VALUE = 1
  ```

### Target version

simplint reads the target version from Ruff's configuration, as `ruff check` does, taking the first that is set:

1. [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) or
   [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version)
1. `project.requires-python`
1. Ruff's default

### Fix

The fix removes `annotations` from the import:

- When `annotations` is the statement's only name, the fix deletes the statement.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.14"
  ```

  ```python report
  # Demo module.
  from __future__ import annotations

  VALUE = 1
  ```

  ```python fixed
  # Demo module.

  VALUE = 1
  ```

- If that statement is the only one in the module, the fix replaces it with `pass`.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.14"
  ```

  ```python report
  from __future__ import annotations
  ```

  ```python fixed
  pass
  ```

- No fix is offered when a statement that imports only `annotations` shares its line with other code.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.14"
  ```

  ```python report
  from __future__ import annotations

  VALUE = 1
  ```

After the fix, code that reads `__annotations__` and expects strings gets evaluated objects
([annotations best practices](https://docs.python.org/3/howto/annotations.html)).
