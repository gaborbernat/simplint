Flags a runtime import that closes a cycle of modules which import each other when they load.

## Why is this bad?

Python adds a module to [`sys.modules`](https://docs.python.org/3/reference/import.html#loading) before executing its
body. If `orders` imports `cart` and `cart` immediately imports a name from `orders`, that name may not exist yet. Which
import fails can depend on which module loads first.

A cycle can also work if the modules read each other's attributes only inside functions. simplint reports these cycles
too because each module depends on the other. The
[Python FAQ](https://docs.python.org/3/faq/programming.html#how-can-i-have-modules-that-mutually-import-each-other)
illustrates both cases.

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

## What it does

simplint checks imports between files in the run. Each module in a cycle gets one report, on its first import that leads
back to it. The message shows the chain of files. For cycles longer than eight modules it shows the first eight, the
number omitted and the file where the cycle closes.

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

  A module that binds `TYPE_CHECKING` to a true constant runs the block, so the name is an ordinary flag and the import
  counts:

  ```python context file=a.py
  import b
  ```

  ```python report file=b.py
  TYPE_CHECKING = True

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
  module-level name the module binds once to one, as `_false = 0` does to keep an import that only tools read. A second
  binding of the name, in any block of the module or in a `global` declaration in a function, makes it an ordinary flag.

  ```python context file=a.py
  import b
  ```

  ```python ok file=b.py
  _false = 0
  if _false:
      import a
  ```

  A second binding in a `try` makes the same name an ordinary flag, so the import under it counts:

  ```python context file=a.py
  import b
  ```

  ```python report file=b.py
  HAS_JSON = False
  try:
      import json

      HAS_JSON = True
  except ImportError:
      pass

  if HAS_JSON:
      import a
  ```

  A `from module import *` rebinds the public module-level names, so it makes every such public constant an ordinary
  flag. A star import alone does not mark an underscore-prefixed name as rebound, even when the imported module's
  `__all__` lists it, so it stays a constant:

  ```python context file=a.py
  import b
  ```

  ```python context file=flags.py
  FLAG = True
  ```

  ```python report file=b.py
  FLAG = False
  from flags import *

  if FLAG:
      import a
  ```

  With an underscore in the name, the same flag stays a constant:

  ```python context file=a.py
  import b
  ```

  ```python context file=flags.py
  FLAG = True
  ```

  ```python ok file=b.py
  _FLAG = False
  from flags import *

  if _FLAG:
      import a
  ```

- **Imports under a [`sys.version_info`](https://docs.python.org/3/library/sys.html#sys.version_info) check that only
  older Pythons than the project's target pass**, such as `if sys.version_info < (3, 8):` for a project that targets
  Python 3.11, or the `else` of `if sys.version_info >= (3, 8):`. A check of the first item, as
  `sys.version_info[0] == 2` or `sys.version_info.major < 3`, counts too.

  ```python context file=a.py
  import b
  ```

  ```python ok file=b.py
  import sys

  if sys.version_info < (3, 8):
      import a
  ```

  The `else` of a check of the first item that holds on Python 3, the only major a supported Python has, never runs:

  ```python context file=a.py
  import b
  ```

  ```python ok file=b.py
  import sys

  if sys.version_info[0] == 3:
      VALUE = 1
  else:
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

- **`lazy` imports**. Python 3.15 can defer loading the imported module until a bound name is first used.

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

Imports that a program runs by name, such as
[`importlib.import_module("a")`](https://docs.python.org/3/library/importlib.html#importlib.import_module), are not in
the graph, and neither is the import of a package's `__init__.py` that Python runs before a submodule.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issues [#2914](https://github.com/astral-sh/ruff/issues/2914) and
  [#970](https://github.com/astral-sh/ruff/issues/970) (pylint `cyclic-import`); pull request
  [#3880](https://github.com/astral-sh/ruff/pull/3880) was closed unmerged.
- Pylint also checks for circular imports with
  [cyclic-import](https://pylint.readthedocs.io/en/latest/user_guide/messages/refactor/cyclic-import.html).
