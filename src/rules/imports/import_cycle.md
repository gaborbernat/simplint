Flags a runtime import that closes a cycle of modules which import each other when they load.

## Why is this bad?

A module runs its imports before its own body. When two modules import each other at load time, whichever loads first
sees the other half-built: a name it imports from the other may not exist yet, and the failure depends on which module
the program imports first. The cycle also ties the two modules together, so neither can be read, tested or moved on its
own.

In the example below, `orders.py` and `cart.py` import each other at the top of their files.

## Example

```python report file=orders.py
from cart import total


def place(items):
    return total(items)
```

```python report file=cart.py
from orders import place


def total(items):
    return sum(items)


def checkout(items):
    return place(items)
```

Import `place` inside `checkout` instead, where it runs after both modules have loaded:

```python context file=orders.py
from cart import total


def place(items):
    return total(items)
```

```python ok file=cart.py
def total(items):
    return sum(items)


def checkout(items):
    from orders import place

    return place(items)
```

## Conflicts with Ruff

Ruff has no lint rule for import cycles, so the two do not overlap.

## What it does

simplint builds a graph of the modules of the run whose imports run when the module loads. A module reports once, on its
first import into a cycle of the graph, and the report names the cycle: a cycle of two modules gives two reports, one in
each module, each on the import that closes the cycle from that module.

### Active on

- **`import` and `from ... import` statements at module level**, including those in a top-level `if` or `try` block and
  in a class body, which also run when the module loads.

  ```python report file=a.py
  import b
  ```

  ```python report file=b.py
  import a
  ```

- **A submodule's import of a name its package's `__init__.py` defines**, when the `__init__.py` imports the submodule:
  the package's `__init__.py` has not finished when the submodule runs. A `from . import name` that names a submodule
  imports that submodule instead.

  ```python report file=pkg/__init__.py
  from .core import run

  VERSION = "1"
  ```

  ```python report file=pkg/core.py
  from . import VERSION


  def run():
      return VERSION
  ```

- **Imports under a platform check**, such as `if sys.platform == "win32":`, since the cycle breaks the program on that
  platform.

  ```python report file=a.py
  import b
  ```

  ```python report file=b.py
  import sys

  if sys.platform == "win32":
      import a
  ```

### Exceptions

- **Imports inside a function**, which run when the function is called, and not when the module loads.

  ```python context file=a.py
  VALUE = 1
  ```

  ```python ok file=b.py
  def load():
      import a

      return a.VALUE
  ```

- **Imports under `if TYPE_CHECKING:`**, which a type checker reads but the interpreter never runs.

  ```python context file=a.py
  VALUE = 1
  ```

  ```python ok file=b.py
  from typing import TYPE_CHECKING

  if TYPE_CHECKING:
      import a
  ```

- **Imports under a [`__main__` guard](https://docs.python.org/3/library/__main__.html#name-main)**, such as
  `if __name__ == "__main__":`, which run only when the file is executed as a script.

  ```python context file=a.py
  import b
  ```

  ```python ok file=b.py
  VALUE = 1

  if __name__ == "__main__":
      import a
  ```

- **Imports under an `if` whose test is false at run time**: a false constant such as `False`, `0` or `None`, or a
  module-level name the module binds once to one, as `_false = 0` does to keep an import that only tools read.

  ```python context file=a.py
  import b
  ```

  ```python ok file=b.py
  _false = 0
  if _false:
      import a
  ```

- **Imports under a `sys.version_info` check that only older Pythons than the project's target pass**, such as
  `if sys.version_info < (3, 8):` for a project that targets Python 3.11, or the `else` of
  `if sys.version_info >= (3, 8):`. A check of the first item, as `sys.version_info[0] == 2` or
  `sys.version_info.major < 3`, counts too.

  ```python context file=a.py
  import b
  ```

  ```python ok file=b.py
  import sys

  if sys.version_info < (3, 8):
      import a
  ```

- **Imports in an `except ImportError:` or `except ModuleNotFoundError:` clause**, a fallback that runs only when the
  import its `try` attempts fails, as when a pure Python module stands in for a missing C accelerator.

  ```python context file=a.py
  import b
  ```

  ```python ok file=b.py
  try:
      from _speedups import parse
  except ImportError:
      from a import parse
  ```

- **Test modules**, which the run tells by their names and the `tests` and `test` directories of the project. Tests
  import each other as fixtures, and the tests of an import system build cycles on purpose.

  ```python context file=tests/helpers.py
  import tests.test_load
  ```

  ```python ok file=tests/test_load.py
  import tests.helpers
  ```

- **`lazy` imports**, which Python 3.15 binds on first use.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.15"
  ```

  ```python context file=a.py
  import b
  ```

  ```python ok file=b.py
  lazy import a
  ```

### Undetected cases

Imports that a program runs by name, such as `importlib.import_module("a")`, are not in the graph, and neither is the
import of a package's `__init__.py` that Python runs before a submodule.

## Other tools

- [pylint](https://github.com/pylint-dev/pylint) reports the same cycles as
  [cyclic-import](https://pylint.readthedocs.io/en/latest/user_guide/messages/refactor/cyclic-import.html), with no fix.
