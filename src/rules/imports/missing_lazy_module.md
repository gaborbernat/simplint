Flags a module-level import that nothing uses while the module loads, when the module's
[`__lazy_modules__`](https://docs.python.org/3.15/reference/simple_stmts.html#compatibility-via-lazy-modules) does not
list it.

## Why is this bad?

Python 3.15 defers an import that `__lazy_modules__` lists until the first use of a name it binds
([PEP 810](https://peps.python.org/pep-0810/)), and older versions ignore the list. An eager import can load a
dependency before any caller needs it. For example, a command that only prints `--help` might still import the library
used to send a request.

## Example

```toml config
[project]
name = "demo"
requires-python = ">=3.10"
```

```python report
# Demo module.
__lazy_modules__ = ["argparse"]

import argparse
import json


def main() -> None:
    parser = argparse.ArgumentParser()
    print(json.dumps(vars(parser.parse_args())))
```

```python fixed
# Demo module.
__lazy_modules__ = ["argparse", "json"]

import argparse
import json


def main() -> None:
    parser = argparse.ArgumentParser()
    print(json.dumps(vars(parser.parse_args())))
```

## Configuration

- The [target Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version) for
  the file. The rule runs only for targets below Python 3.15. On 3.15 and later, use `lazy` directly.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.15"
  ```

  ```python ok
  __lazy_modules__ = []

  import json


  def dump() -> str:
      return json.dumps(1)
  ```

- [`require-lazy-modules`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#require-lazy-modules):
  `true` checks every module, and the fix adds `__lazy_modules__` where a module has none. The default is `false`, which
  checks only the modules that assign `__lazy_modules__`, so a project adopts lazy imports one module at a time.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.10"

  [tool.simplint.imports]
  require-lazy-modules = true
  ```

  ```python report
  # Demo module.
  import json


  def dump() -> str:
      return json.dumps(1)
  ```

  ```python fixed
  # Demo module.
  __lazy_modules__ = ["json"]

  import json


  def dump() -> str:
      return json.dumps(1)
  ```

## What it does

### Active on

- **An `import` or `from ... import` directly in the module body**, when every import of that module at module level
  binds names that only run later: in a function body, under `if __name__ == "__main__":`, under `if TYPE_CHECKING:`, or
  in an annotation, which Python 3.15 evaluates on access. A relative import is listed as `f"{__spec__.parent}.name"`,
  the form [flake8-lazy](https://github.com/henryiii/flake8-lazy) writes, which follows the package when it is renamed
  or vendored.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.10"
  ```

  ```python context file=pkg/__init__.py
  # A package.
  ```

  ```python context file=pkg/helper.py
  VALUE = 1
  ```

  ```python report file=pkg/consumer.py
  __lazy_modules__ = []

  from .helper import VALUE


  def value() -> int:
      return VALUE
  ```

  ```python fixed file=pkg/consumer.py
  __lazy_modules__ = [f"{__spec__.parent}.helper"]

  from .helper import VALUE


  def value() -> int:
      return VALUE
  ```

### Exceptions

- **A name used while the module loads**: at module level, in a class body, a decorator or a default value, in a lambda
  outside a function, in a function or class that module-level code calls, in the methods of a class that a module-level
  class uses as its metaclass, in the `__init_subclass__` of a class it subclasses, or in an annotation a decorator such
  as FastAPI's reads. A class call counts the methods of the classes it inherits from too. A lazy import of it would
  load at once. Ruff's [TID255](https://docs.astral.sh/ruff/rules/lazy-import-immediately-resolved/) also checks for
  immediate use, though it can miss reads by a decorator.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.10"
  ```

  ```python ok
  __lazy_modules__ = []

  import pathlib
  import re


  def compile_all() -> list[re.Pattern[str]]:
      return [re.compile(pattern) for pattern in ("a", "b")]


  ROOT = pathlib.Path(".")
  PATTERNS = compile_all()
  ```

- **An unused, unexported name or a name that is rebound**. An unread import may register a type or plugin as a side
  effect. Making it lazy could prevent registration entirely.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.10"
  ```

  ```python ok
  __lazy_modules__ = []

  import json.tool
  ```

- **Modules Python loads at startup**, such as `os`, `sys` and `abc`: they have loaded before the module runs, so a lazy
  import of them saves nothing. So does `import pkg` of a package that holds the file, while `from pkg import name` can
  still load a submodule on first use and is reported.

- **Imports above `__lazy_modules__`**: they load before the list exists, so listing them defers nothing.
  [`ineffective-lazy-declaration`](SMP316.html) reports a list placed after an import it names.

- **Imports in a block, a `try`, a function or a class**: only statements directly in the module body are reported.
  `try` imports stay eager under the list, and the rest usually depend on a condition.

- **A `__lazy_modules__` that is not a literal**, such as a list built by a function, or one that the module changes
  after assigning it: the rule cannot tell what it holds.

### Fix

One unsafe fix adds all the missing modules. It preserves an existing sort order or appends the entries if the list is
unsorted. A new list goes above the first ordinary import, after the docstring and future imports. An unparenthesized
tuple or a comment that cannot stay with its entry prevents the rewrite.

Review what each module does at import time. On Python 3.15, an import that registers a handler may now register it
later or never.

## Other tools

- [flake8-lazy](https://github.com/henryiii/flake8-lazy) reports missing standard library entries as `LZY101` and
  third-party or local modules as `LZY102`.
- Ruff's [TID254](https://docs.astral.sh/ruff/rules/lazy-import-mismatch/) enforces configured `require-lazy` and
  `ban-lazy` policies. [Issue #29175](https://github.com/astral-sh/ruff/issues/29175) requests a fix that writes
  `__lazy_modules__` when targeting older versions.
- [Lifeguard](https://github.com/meta/Lifeguard) analyzes import-time effects across modules. Its JSON output identifies
  modules eligible for lazy loading and constraints on how to load them. That output can drive a
  [`sys.set_lazy_imports_filter()`](https://docs.python.org/3.15/library/sys.html#sys.set_lazy_imports_filter) callback.
