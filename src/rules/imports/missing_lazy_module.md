Flags a module-level import that nothing uses while the module loads, when the module's
[`__lazy_modules__`](https://docs.python.org/3.15/reference/simple_stmts.html#compatibility-via-lazy-modules) does not
list it.

## Why is this bad?

Python 3.15 defers an import that `__lazy_modules__` lists until the first use of a name it binds
([PEP 810](https://peps.python.org/pep-0810/)), and older versions ignore the list. An unlisted import that the module
uses only inside functions still loads in full each time the module is imported, so every caller pays for it, including
a command line run that only prints `--help`.

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

- The Python version the code targets, taken from
  [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) in `[tool.ruff]`, falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python);
  see [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version). From Python
  3.15 the `lazy` keyword spells a lazy import, which Ruff's
  [`lazy-import-mismatch`](https://docs.astral.sh/ruff/rules/lazy-import-mismatch/) (`TID254`) asks for, so the rule
  stays silent there.

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
  load at once. Ruff's
  [`lazy-import-immediately-resolved`](https://docs.astral.sh/ruff/rules/lazy-import-immediately-resolved/) (`TID255`)
  reports a listed import used this way.

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

- **An import whose name nothing reads and no other module imports**: it runs for what loading the module does, such as
  registering a type or a plugin, which a lazy import would never do.

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

The fix adds every missing module to the list in one edit, sorted as [`unsorted-lazy-modules`](SMP315.html) asks when
the list is sorted, and at its end otherwise. A new list goes above the first import, below the docstring and the
`from __future__` imports. The fix is unsafe: on Python 3.15 the module then loads on first use, so anything its import
does, such as registering a handler, happens later or not at all. A comment inside the list other than at the end of an
entry's line withholds the fix.

## Other tools

- **[flake8-lazy](https://github.com/henryiii/flake8-lazy):** `LZY101` and `LZY102`. It treats every function body as
  deferred, so it also lists an import that a function called at module level uses, and one imported only for what
  loading it does.
- **[Ruff](https://github.com/astral-sh/ruff):** [`TID254`](https://docs.astral.sh/ruff/rules/lazy-import-mismatch/)
  asks for `lazy` on the imports its `require-lazy` setting names, from Python 3.15, without looking at how they are
  used. Open issue [#29175](https://github.com/astral-sh/ruff/issues/29175) asks it to write `__lazy_modules__` for
  older versions.
- **[Lifeguard](https://github.com/meta/Lifeguard):** Meta's analyzer follows imports across a whole project, finds
  module-level side effects that break when a module loads lazily, such as a registry filled on import or
  [`sys.modules`](https://docs.python.org/3/library/sys.html#sys.modules) writes, and writes the modules that are safe
  to defer as JSON for
  [`sys.set_lazy_imports_filter()`](https://docs.python.org/3.15/library/sys.html#sys.set_lazy_imports_filter). This
  rule reads one file at a time and judges an import only by where the file uses its names, so it cannot see what
  loading the imported module does. Lifeguard answers which modules are safe to defer; this rule keeps each module's
  `__lazy_modules__` complete once the author has decided.
