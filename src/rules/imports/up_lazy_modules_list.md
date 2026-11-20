Flags a module-level [`__lazy_modules__`](https://docs.python.org/3/reference/datamodel.html#module.__lazy_modules__)
list whose entries are plain imports in the same module, which Python 3.15 spells as `lazy import` on each import
([PEP 810](https://peps.python.org/pep-0810/)).

## Why is this bad?

[`__lazy_modules__`](https://docs.python.org/3/reference/simple_stmts.html#compatibility-via-lazy-modules) lets the same
file parse on older versions and request lazy imports on Python 3.15. Older interpreters ignore the declaration. Once a
project requires Python 3.15, `lazy` on each import makes it clear which imports may be deferred.

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

- The [target Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version) for
  the file

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
  level names one of its entries. An entry spelled `f"{__spec__.parent}.name"`, as flake8-lazy writes one for a relative
  import, names the module relative to the file's package. A relative import matches by the module it names after
  `from`, so `from . import helper` matches a listed `pkg`, and not a listed `pkg.helper`. The report sits on the list.

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
  # Demo module.
  __lazy_modules__ = [f"{__spec__.parent}.helper"]

  from .helper import VALUE

  print(VALUE)
  ```

  ```python fixed file=pkg/consumer.py
  # Demo module.

  lazy from .helper import VALUE

  print(VALUE)
  ```

### Exceptions

- **Values that are not literals**, such as a list built from a comprehension or a name, or a list the module names
  again after assigning it: the rule cannot read the entries, so it does not report them.

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

The rule marks the fix safe. It preserves existing `lazy` keywords, adds `lazy` to matching imports directly in the
module body and removes the declaration. Removing the declaration also removes any comments inside it.

Every entry must match an import, and the declaration must occupy its own lines. The rule leaves `import json, os` alone
if only `json` is listed, because `lazy` would apply to both. It also withholds the fix if a matching eager import
occurs above the declaration or another import depends on the declaration inside a module-level block. Such blocks
include `if`, `with`, loops, `match`, a `try` statement's `finally` and its `else` when there is no `finally`.

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

## Other tools

- [Ruff issue #29175](https://github.com/astral-sh/ruff/issues/29175) requests adding `__lazy_modules__` for files
  targeting Python before 3.15.
- [Lifeguard](https://github.com/meta/Lifeguard) analyzes which modules can load lazily and reports constraints for a
  [`sys.set_lazy_imports_filter()`](https://docs.python.org/3.15/library/sys.html#sys.set_lazy_imports_filter) callback.
  This rule converts lazy imports the module already declares.
