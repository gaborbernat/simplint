Flags an `if TYPE_CHECKING:` block that holds only imports and no `else`, where Python 3.15 can spell each import as a
`lazy` import. The guard is then only needed for names that are not imported at runtime, which `lazy` covers.

## Why is this bad?

The guard keeps the imports out of the running program, so the names they bind exist only for type checkers, and code
that reads them at runtime fails. A `lazy` import binds the name at module level and runs the import on first use
([PEP 810](https://peps.python.org/pep-0810/)), so the guard is no longer needed and a name used at runtime resolves.

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

- The Python version the code targets, taken from
  [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) in `[tool.ruff]`, falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python);
  see [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

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

- **`if TYPE_CHECKING:` (or `typing.TYPE_CHECKING`) at module level**, with a body of `import` and `from ... import`
  statements only and no `else` or `elif`. The report sits on the condition. The fix is unsafe: a `lazy` import loads
  the module on first use, so a name a program never touches costs nothing, while a name it does touch runs the import
  then. An import cycle or a module missing at runtime, such as `_typeshed`, then surfaces at that point instead of
  never.

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

- **Blocks inside another statement**, such as a `try` or a function: `lazy` imports are not allowed there, so the rule
  does not look at them.

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

The fix is unsafe, as described under "Active on" above. It replaces the block with one `lazy` import per statement,
and, when `TYPE_CHECKING` is then unused, removes its `from typing import TYPE_CHECKING` line. A block with a comment in
it is reported without a fix, because the fix would drop the comment. A block importing `_typeshed` is too.

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
