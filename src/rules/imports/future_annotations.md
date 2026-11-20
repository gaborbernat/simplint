Flags `from __future__ import annotations` when the target version is Python 3.14 or newer.

## Why is this bad?

Python 3.14
[defers annotation evaluation](https://docs.python.org/3/whatsnew/3.14.html#whatsnew314-deferred-annotations) by
default. With the future import, reading `__annotations__` still returns strings. Without it, reading the attribute
evaluates the annotations and can raise if a name is missing. [PEP 649](https://peps.python.org/pep-0649/) describes
this behavior.

[PEP 749](https://peps.python.org/pep-0749/#the-future-of-from-future-import-annotations) allows deprecation from the
first release after Python 3.13 reaches end of life. Once compiling the import starts emitting a
[`DeprecationWarning`](https://docs.python.org/3/builtins/exceptions.html#DeprecationWarning), it will remain available
for at least two releases.

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
removes. If both rules apply to a file targeting Python 3.14 or newer, simplint stops before linting. Remove the
required import or disable this rule.

## Configuration

- The following settings can select the target version. See \[Python version\](https for command-line overrides, other
  project declarations and the default.//simplint.readthedocs.io/en/latest/configuration/project.html#python-version):

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
  [`if TYPE_CHECKING:`](https://docs.python.org/3/library/typing.html#typing.TYPE_CHECKING) raises
  [`NameError`](https://docs.python.org/3/builtins/exceptions.html#NameError) there.

  The fix is also unsafe when an annotation unions a string with a type, such as `"Node" | None`, which raises once
  evaluated, or when an import sits under a module-level `if`, which may leave its name unbound. Otherwise the fix is
  safe.

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

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.14"
  ```

  ```python report
  # Demo module.
  from __future__ import annotations


  class Node:
      pass


  def find(parent: "Node" | None) -> None:
      pass
  ```

  ```python fixed
  # Demo module.


  class Node:
      pass


  def find(parent: "Node" | None) -> None:
      pass
  ```

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.14"
  ```

  ```python report
  from __future__ import annotations
  import sys

  if sys.platform == "win32":
      from os import PathLike


  def path(value: PathLike) -> None:
      pass
  ```

  ```python fixed
  import sys

  if sys.platform == "win32":
      from os import PathLike


  def path(value: PathLike) -> None:
      pass
  ```

- **Annotations that name the class itself or a later module-level name**: the class body binds neither, so the rule
  still reports the module.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.14"
  ```

  ```python report
  from __future__ import annotations


  class Node:
      def add(self, other: Node) -> Tree:
          return self


  class Tree:
      pass
  ```

### Exceptions

- **Classes whose annotations read a name their body binds**: lazily evaluated annotations see the class namespace, so a
  method named `list` makes `list[str]` mean the method. With the import, these annotations stay strings until the
  caller decides how to evaluate them. simplint skips the whole module if it finds such a class.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.14"
  ```

  ```python ok
  from __future__ import annotations


  class Repo:
      def list(self) -> list[str]:
          return []


  class Point:
      float: float = 0.0
  ```

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

- No fix is offered when a comment sits on the statement's lines, since the fix would delete it.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.14"
  ```

  ```python report
  from __future__ import annotations  # kept for the docs build

  VALUE = 1
  ```

  ```python fixed
  from __future__ import annotations  # kept for the docs build

  VALUE = 1
  ```

After the fix, code that reads `__annotations__` and expects strings gets evaluated objects
([annotations best practices](https://docs.python.org/3/howto/annotations.html)).

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issue [#20739](https://github.com/astral-sh/ruff/issues/20739).
