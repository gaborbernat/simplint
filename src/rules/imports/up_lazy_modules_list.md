Flags a module-level [`__lazy_modules__`](https://docs.python.org/3/reference/datamodel.html#module.__lazy_modules__)
list whose entries are plain imports in the same module, which Python 3.15 spells as `lazy import` on each import
([PEP 810](https://peps.python.org/pep-0810/)).

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

- **Imports in the body or the handlers of a `try`, in a function, or in a class**: CPython loads them eagerly even
  under the list, so the rule does not count them as matches.

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
entry has a matching import that can take `lazy`, and the list is alone on its lines. An import that is `lazy` already
needs no second `lazy`, and counts as matching.

An import that names a listed module together with other modules, such as `import json, os`, is reported with no fix:
`lazy` would make the other modules lazy too. A statement that is not `lazy`-able keeps its entry, so the fix does not
apply.

The fix is also withheld when a listed module is imported again where the list keeps the import lazy and the fix would
make it eager: under an `if`, `with`, `for`, `while` or `match` at module level, in the `else` of a `try` without a
`finally`, or in its `finally`. A relative import is held by the module it names after `from`, so `from . import helper`
is held by a listed `pkg`, and not by a listed `pkg.helper`. The list is what keeps those imports lazy, and deleting it
would make them eager.

```toml config
[project]
name = "demo"
requires-python = ">=3.15"
```

```python report
# Demo module.
__lazy_modules__ = ["json"]

import json

if __name__ == "__main__":
    import json as parser
```

```python fixed
# Demo module.
__lazy_modules__ = ["json"]

import json

if __name__ == "__main__":
    import json as parser
```

```toml config
[project]
name = "demo"
requires-python = ">=3.15"
```

```python report
# Demo module.
__lazy_modules__ = ["json"]

lazy import json

print(json.dumps([1]))
```

```python fixed
# Demo module.

lazy import json

print(json.dumps([1]))
```

```toml config
[project]
name = "demo"
requires-python = ">=3.15"
```

```python report
__lazy_modules__ = ["json"]

import json

try:
    pass
except ImportError:
    pass
else:
    import json as fallback
```

```python fixed
__lazy_modules__ = ["json"]

import json

try:
    pass
except ImportError:
    pass
else:
    import json as fallback
```

```toml config
[project]
name = "demo"
requires-python = ">=3.15"
```

```python context file=pkg/__init__.py
# A package.
```

```python context file=pkg/helper.py
VALUE = 1
```

```python report file=pkg/consumer.py
__lazy_modules__ = ["pkg"]

import pkg
from . import helper

print(pkg, helper.VALUE)
```

```python fixed file=pkg/consumer.py
__lazy_modules__ = ["pkg"]

import pkg
from . import helper

print(pkg, helper.VALUE)
```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet. Open issue
  [#29175](https://github.com/astral-sh/ruff/issues/29175) asks for the reverse, adding `__lazy_modules__` for Python
  before 3.15.
- **[Lifeguard](https://github.com/meta/Lifeguard):** Meta's analyzer for [PEP 810](https://peps.python.org/pep-0810/)
  lazy imports works across a whole project. It follows imports, finds module-level side effects that break when a
  module loads lazily, and writes the modules that are safe to load lazily as JSON for
  [`sys.set_lazy_imports_filter()`](https://docs.python.org/3.15/library/sys.html#sys.set_lazy_imports_filter). Use it
  when a large codebase starts adopting lazy imports. This rule stays within one file and judges no import's safety: it
  rewrites a `__lazy_modules__` list the author already wrote as `lazy import` statements, which load the same modules
  lazily.
