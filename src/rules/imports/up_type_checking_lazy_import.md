Flags module-level `if TYPE_CHECKING:` blocks that contain only imports and have no `else` or `elif`. On Python 3.15,
those imports can use [`lazy`](https://docs.python.org/3/reference/simple_stmts.html#lazy-imports).

## Why is this bad?

The guard keeps the imports out of the running program, so the names they bind exist only for type checkers, and code
that reads them at runtime fails, such as code that
[evaluates annotations](https://docs.python.org/3/howto/annotations.html#manually-un-stringizing-stringized-annotations).
A `lazy` import binds the name at module level and runs the import on first use
([PEP 810](https://peps.python.org/pep-0810/)). A name used only in annotations loads nothing until something
[reads the annotations](https://peps.python.org/pep-0810/#how-do-explicit-lazy-imports-interact-with-pep-649-and-pep-749),
and a name used at runtime resolves. The PEP names
[removing `TYPE_CHECKING` guards](https://peps.python.org/pep-0810/#what-about-type-annotations-and-type-checking-imports)
as one of its uses.

## Example

```toml config
[project]
name = "demo"
requires-python = ">=3.15"
```

```python report
# Demo module.
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence


def first(values: Sequence[int]) -> int:
    return values[0]
```

```python fixed
# Demo module.

lazy from collections.abc import Sequence


def first(values: Sequence[int]) -> int:
    return values[0]
```

## Configuration

- The [target Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version) for
  the file

  ```toml config
  [tool.ruff]
  target-version = "py314"
  ```

  ```python ok
  from typing import TYPE_CHECKING

  if TYPE_CHECKING:
      from collections.abc import Sequence
  ```

## What it does

### Active on

- **`if TYPE_CHECKING:` (or
  [`typing.TYPE_CHECKING`](https://docs.python.org/3/library/typing.html#typing.TYPE_CHECKING)) at module level**, with
  a body of `import` and `from ... import` statements only and no `else` or `elif`. The report sits on the condition.
  The fix is unsafe because reading a name at runtime may now load its module and expose an import error or cycle.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.15"
  ```

  ```python report
  # Demo module.
  from typing import TYPE_CHECKING

  if TYPE_CHECKING:
      from os import PathLike


  def path(value: PathLike) -> None:
      pass
  ```

  ```python fixed
  # Demo module.

  lazy from os import PathLike


  def path(value: PathLike) -> None:
      pass
  ```

### Exceptions

- **Blocks with an `else`, an `elif`, or other statements**: the rule needs a block that only imports.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.15"
  ```

  ```python ok
  from typing import TYPE_CHECKING

  if TYPE_CHECKING:
      from os import PathLike
  else:
      PathLike = str
  ```

- **Imports of `from __future__` and `from x import *`**: `lazy` has no form for them.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.15"
  ```

  ```python ok
  from typing import TYPE_CHECKING

  if TYPE_CHECKING:
      from os import *
  ```

- **Nested blocks**. The rule checks only guards directly in the module body. Python forbids `lazy` inside functions and
  `try` bodies.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.15"
  ```

  ```python ok
  from typing import TYPE_CHECKING

  try:
      if TYPE_CHECKING:
          from os import PathLike
  except ImportError:
      pass
  ```

### Fix

The fix moves each import out of the guard and adds `lazy` where needed. It also removes a standalone
`from typing import TYPE_CHECKING` if this guard was its only use. Imports of `_typeshed` get no fix because that module
is for type checkers. Comments inside the block, an inline body or imports that share a line also prevent the fix.

```toml config
[project]
name = "demo"
requires-python = ">=3.15"
```

```python report
# Demo module.
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    lazy import json
```

```python fixed
# Demo module.

lazy import json
```

```toml config
[project]
name = "demo"
requires-python = ">=3.15"
```

```python report
# Demo module.
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from os import PathLike
```

```python fixed
# Demo module.

lazy from os import PathLike
```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issue [#29378](https://github.com/astral-sh/ruff/issues/29378).
  [TID254](https://docs.astral.sh/ruff/rules/lazy-import-mismatch/), with `require-lazy` set, can add `lazy` to a
  matching single-name import inside the guard. It keeps the guard.
