Flags a [`__lazy_modules__`](https://docs.python.org/3.15/reference/simple_stmts.html#compatibility-via-lazy-modules)
entry or a [`lazy`](https://docs.python.org/3.15/reference/simple_stmts.html#lazy-imports) import that defers nothing.

## Why is this bad?

A declaration that defers nothing tells the reader a module loads on first use when it loads with the module, or never
loads at all. Python 3.15 checks `__lazy_modules__` against the absolute name of each module-level import it runs
([PEP 810](https://peps.python.org/pep-0810/)), so an entry that matches no such import is silently ignored, and a
module that another import has already loaded has nothing left to defer.

## Example

```python report
__lazy_modules__ = ["json", "pathlib", "json"]

import json


def dump() -> str:
    return json.dumps(1)
```

```python fixed
__lazy_modules__ = ["json"]

import json


def dump() -> str:
    return json.dumps(1)
```

## What it does

### Active on

- **An entry listed twice.** The fix removes the later one, and is safe.

- **An entry no module-level import names**, including one whose module is imported only inside a `try`, which CPython
  keeps eager, or only under `if TYPE_CHECKING:`, which never runs. `import a.b` is named by `"a.b"` and not by `"a"`,
  and `from a import b` by `"a"`. The fix removes the entry, and is safe.

- **An entry for a module imported only with `lazy`**, which defers it without the list. The fix removes the entry, and
  is safe.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.15"
  ```

  ```python report
  __lazy_modules__ = ["json"]

  lazy import json
  ```

  ```python fixed
  __lazy_modules__ = []

  lazy import json
  ```

- **An entry or a `lazy` import of a module that has loaded before the file runs**: a module Python loads at startup,
  such as `os` or `sys`, or with `import pkg`, a package that holds the file. `from pkg import name` can still load a
  submodule on first use, so it keeps its entry. The fix removes the entry or the `lazy`, and is safe.

  ```python report
  __lazy_modules__ = ["os"]

  import os


  def cwd() -> str:
      return os.getcwd()
  ```

  ```python fixed
  __lazy_modules__ = []

  import os


  def cwd() -> str:
      return os.getcwd()
  ```

- **An entry written as a relative name**, such as `".helper"`: no absolute name equals it. The fix writes the entry as
  `f"{__spec__.parent}.helper"`, which names the module relative to the file's package, and is unsafe: the import it
  then matches loads on first use.

  ```python context file=pkg/__init__.py
  # A package.
  ```

  ```python context file=pkg/helper.py
  VALUE = 1
  ```

  ```python report file=pkg/consumer.py
  __lazy_modules__ = [".helper"]

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

- **A `__lazy_modules__` assigned after an import it lists**: that import ran eagerly before the list existed. The
  report sits on the assignment, and the fix moves it above the first import. The fix is unsafe: the import then loads
  on first use. It is withheld when a comment sits above the list, when the list shares a line with other code, and when
  the first import shares a line with an earlier statement, such as a `from __future__` import.

  ```python report
  import json

  __lazy_modules__ = ["json"]


  def dump() -> str:
      return json.dumps(1)
  ```

  ```python fixed
  __lazy_modules__ = ["json"]

  import json


  def dump() -> str:
      return json.dumps(1)
  ```

- **A `lazy` import of a module another statement directly in the module body imports eagerly**: the eager import loads
  it while the module loads. The report has no fix, since only the author knows which of the two should change.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.15"
  ```

  ```python report
  import json
  lazy from json import dumps
  ```

### Exceptions

- **A `__lazy_modules__` that is not a literal**, such as a list built by a function, or one that the module changes
  after assigning it: the rule cannot tell what it holds.

  ```python ok
  __lazy_modules__ = [name for name in ("json",)]
  ```

- **An entry based on `__spec__.parent` outside a package**. simplint cannot resolve it to an absolute name. A plain
  string such as `".helper"` still triggers a report.

### Fix

A fix that edits the list rewrites it, keeping a comment at the end of an entry's line with its entry. Any other comment
inside the list prevents that rewrite. The rule also leaves unparenthesized tuples alone and keeps the last item in a
set because `{}` would create a dictionary. A `lazy` import that names more than one module keeps its `lazy`, since
removing it would make the other modules eager too.

## Other tools

- **[flake8-lazy](https://github.com/henryiii/flake8-lazy):** `LZY202` (never imported), `LZY203` (duplicate), `LZY204`
  (assigned after the import), `LZY205` (relative name), `LZY302` (listed and `lazy`), `LZY303` (imported eagerly and
  lazily) and `LZY402` (enclosing package), as diagnostics. The standalone runner can regenerate the list with
  `--apply=list`.
- **[Ruff](https://github.com/astral-sh/ruff):**
  [`lazy-import-immediately-resolved`](https://docs.astral.sh/ruff/rules/lazy-import-immediately-resolved/) (`TID255`)
  reports a lazy import that module-level code uses at once, including imports deferred by a literal `__lazy_modules__`
  declaration.
- **[Lifeguard](https://github.com/meta/Lifeguard):** finds module-level side effects that break when a module loads
  lazily, across a whole project. This rule checks only that each declaration in a file matches an import the file
  defers.
