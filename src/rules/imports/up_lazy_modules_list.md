Flags a module-level `__lazy_modules__` list whose entries are plain imports in the same module, which Python 3.15
spells as `lazy import` on each import.

## Why is this bad?

`__lazy_modules__` is the compatibility spelling of lazy imports for code that still supports older versions, where the
list is the only way to ask for lazy loading without a syntax error. A project that targets Python 3.15 has the `lazy`
keyword, which puts the request on each import where a reader sees it. The list then only tells readers which imports
are lazy, one line away from the imports it governs.

## Example

```toml config
[project]
name = "demo"
requires-python = ">=3.15"
```

```python report
# Demo module.
__lazy_modules__ = ["json", "pathlib"]

import json
import pathlib

print(json.dumps([1]), pathlib.Path("demo").name)
```

```python fixed
# Demo module.

lazy import json
lazy import pathlib

print(json.dumps([1]), pathlib.Path("demo").name)
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
  __lazy_modules__ = ["json"]

  import json

  print(json.dumps([1]))
  ```

## What it does

### Active on

- **A module-level `__lazy_modules__` assigned a list, tuple or set of string literals**, when some `import` at module
  level names one of its entries. The report sits on the list.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.15"
  ```

  ```python report
  __lazy_modules__ = ["json"]

  import json

  print(json.dumps([1]))
  ```

### Exceptions

- **Values that are not literals**, such as a list built from a comprehension or a name: the rule cannot read the
  entries, so it does not report them.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.15"
  ```

  ```python ok
  __lazy_modules__ = [name for name in ("json",)]

  import json
  ```

- **Imports inside a `try`, a function or a class**: they are not module-level imports, so the rule does not look at
  them.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.15"
  ```

  ```python ok
  __lazy_modules__ = ["json"]

  try:
      import json
  except ImportError:
      json = None
  ```

### Fix

The fix is safe. It puts `lazy` in front of each matching import and deletes the list. It is offered only when every
entry has a matching import that can take `lazy`, and the list is alone on its lines.

An import that names a listed module together with other modules, such as `import json, os`, is reported with no fix:
`lazy` would make the other modules lazy too. A statement that is not `lazy`-able keeps its entry, so the fix does not
apply.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issue [#29175](https://github.com/astral-sh/ruff/issues/29175).
