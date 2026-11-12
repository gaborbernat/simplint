Flags public module-level names that another file in the same run imports but `__all__` does not list.

## Why is this bad?

`__all__` lists the names that make up a module's public interface
([PEP 8](https://peps.python.org/pep-0008/#public-and-internal-interfaces)). A name other modules depend on but
`__all__` omits looks private, so a later change can remove or rename it without anyone noticing the importers.

In the example below, `cli.py` imports `LIMIT` and `shared` from `config.py`, yet `config.py` lists neither, so nothing
in `config.py` tells its next editor that `cli.py` relies on them.

## Example

```python context file=cli.py
from config import LIMIT, local, shared

print(LIMIT, local(), shared())
```

```python report file=config.py
LIMIT = 3


def shared() -> int:
    return LIMIT


def local() -> int:
    return 1


__all__ = ["local"]  # cli.py also imports LIMIT and shared
```

Use instead:

```python context file=cli.py
from config import LIMIT, local, shared

print(LIMIT, local(), shared())
```

```python ok file=config.py
LIMIT = 3


def shared() -> int:
    return LIMIT


def local() -> int:
    return 1


__all__ = ["LIMIT", "local", "shared"]
```

## Conflicts with Ruff

Ruff's [unused-import](https://docs.astral.sh/ruff/rules/unused-import/) (F401) reports an import this rule also reports
when another module imports it, and its safe fix deletes that import. Ruff cannot see the importer, so after
`ruff check --fix` the importer fails with an `ImportError`, and neither tool reports anything. The run does not stop
for this pair, since the fixes do not undo each other: once this rule's fix lists the name, F401 stops reporting it.

Keep the import with `# noqa: F401  # <reason>` on its line: F401 respects the comment, and this rule's fix removes it
once it lists every name on that line. Without the comment, `simplint check --fix` must run before `ruff check --fix`,
and it needs `--unsafe-fixes` when the module has no `__all__` yet, since that fix creates one.

## What it does

### Active on

A public module-level name that [`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package)
does not list is reported when another file in the same run imports it in one of these ways, or when a listed name's
signature reaches it:

- **`from module import name`**, absolute or relative: the importer names the module and the name directly.

  ```python report file=config.py
  LIMIT = 3


  def local() -> int:
      return 1


  __all__ = ["local"]
  ```

  ```python context file=cli.py
  from config import LIMIT

  print(LIMIT)
  ```

- **`module.name` access after `import module`**: the importer reads the name as an attribute of the module.

  ```python report file=config.py
  LIMIT = 3


  def local() -> int:
      return 1


  __all__ = ["local"]
  ```

  ```python context file=cli.py
  import config

  print(config.LIMIT)
  ```

- **[Entry points](https://packaging.python.org/en/latest/specifications/pyproject-toml/#entry-points)** declared in
  `pyproject.toml` under `[project.scripts]`, `[project.gui-scripts]` or `[project.entry-points]`: installers and plugin
  hosts import the name from outside the code.

  ```toml config
  [project]
  name = "demo"

  [project.scripts]
  run = "config:main"
  ```

  ```python report file=config.py
  def main() -> None:
      pass


  def helper() -> int:
      return 1


  __all__ = ["helper"]
  ```

- **String-named handlers** passed to
  [`pkgutil.resolve_name`](https://docs.python.org/3/library/pkgutil.html#pkgutil.resolve_name) as `module:attribute`,
  or stored with `parser.set_defaults(handler="module:attribute")`: the dispatch reads the named attribute at runtime.

  ```python report file=config.py
  def main() -> None:
      pass


  __all__ = []
  ```

  ```python context file=cli.py
  import pkgutil

  main = pkgutil.resolve_name("config:main")
  ```

- **Tool references in `pyproject.toml`**: setuptools reads `[tool.setuptools.dynamic]` `attr` values and
  `[tool.setuptools.cmdclass]` classes; mypy calls the function named by `[tool.mypy] plugins` (`plugin` by default);
  Pylint calls `register` in `[tool.pylint.main] load-plugins`. Legacy `[tool.poetry.plugins]` values are entry points.
  A pytest plugin loaded with `-p` in `[tool.pytest.ini_options] addopts` or `[tool.pytest] addopts` exposes its public
  hooks by name.

  ```toml config
  [tool.mypy]
  plugins = ["config"]
  ```

  ```python report file=config.py
  def plugin(version: str) -> None:
      pass


  __all__ = []
  ```

- **Build backend hooks** of the module that `build-backend` names in `[build-system]`: a build frontend calls the PEP
  517 and PEP 660 hooks by name, so the names are used from outside the code. When the backend is `pdm.backend`, its
  local `pdm_build.py` hook functions are also called by name; `[tool.pdm.build] custom-hook` may name another file.

  ```toml config
  [build-system]
  build-backend = "build_backend"
  requires = ["setuptools"]
  backend-path = ["."]
  ```

  ```python report file=build_backend.py
  def build_wheel(wheel_directory: str) -> str:
      return wheel_directory


  __all__ = []
  ```

- **Declarations in the module's `.pyi` stub**: a stub publishes the names it lists.

  ```python report file=config.py
  LIMIT = 3


  def local() -> int:
      return 1


  __all__ = ["local"]
  ```

  ```python context file=config.pyi
  LIMIT: int
  ```

- **Imports the module never uses**: such an import is there for its importers, so it counts as a re-export and is
  reported.

  ```python report file=config.py
  from os import sep


  def local() -> int:
      return 1


  __all__ = ["local"]
  ```

  ```python context file=cli.py
  from config import sep

  print(sep)
  ```

- **Classes and aliases in the signature of a listed name**: the annotations, bases and bounds of an exported function,
  or of a public class, that refer to a class or alias make it public API, though no file imports it. Such a name is
  reached through the same places of every class and alias the signature reaches, and it counts only when its name is
  public.

  ```python report file=config.py
  Level = int


  def local() -> Level:
      return 1


  __all__ = ["local"]
  ```

  ```python fixed file=config.py
  Level = int


  def local() -> Level:
      return 1


  __all__ = [
      "Level",
      "local",
  ]
  ```

  ```python context file=cli.py
  from config import local

  print(local())
  ```

### Exceptions

- **`__main__.py` files**: these are entry points, so imports by tests or runners do not make their names a public API.

  ```python ok file=app/__main__.py
  LIMIT = 3
  __all__ = []
  ```

  ```python context file=cli.py
  from app.__main__ import LIMIT

  print(LIMIT)
  ```

- **Names used only by tests**: a test of a module may inspect its implementation without making that name part of the
  module's public interface. A production importer still makes the name reportable.

  ```python ok file=config.py
  class Parser:
      pass


  __all__ = []
  ```

  ```python context file=test_config.py
  from config import Parser
  ```

- **Names only tests import from a module production code loads as a value**: production code that passes a module on,
  as `CONFIG = settings` does, may call any of its names through that value, so a name only tests import is still not
  reported.

  ```python ok file=config.py
  class Parser:
      pass


  __all__ = []
  ```

  ```python context file=cli.py
  import config as settings

  CONFIG = settings
  ```

  ```python context file=test_config.py
  from config import Parser
  ```

- **Names starting with `_`**: the underscore already marks them as internal.

  ```python ok file=config.py
  _LIMIT = 3
  __all__ = []
  ```

  ```python context file=cli.py
  from config import _LIMIT

  print(_LIMIT)
  ```

- **`__init__.py` files**: these are not checked.

  ```python ok file=pkg/__init__.py
  LIMIT = 3
  __all__ = []
  ```

  ```python context file=cli.py
  from pkg import LIMIT

  print(LIMIT)
  ```

- **Private names in the signature of an exported name**: a type a listed name's annotations refer to is public API only
  when its name is public, and the underscore marks a private one as internal.

  ```python ok file=config.py
  _Level = int


  def local() -> _Level:
      return 1


  __all__ = ["local"]
  ```

  ```python context file=cli.py
  from config import local

  print(local())
  ```

- **Test modules**, as defined in [test-class](SMP801.html): these are not checked.

  ```python ok file=test_config.py
  LIMIT = 3
  __all__ = []
  ```

  ```python context file=cli.py
  from test_config import LIMIT

  print(LIMIT)
  ```

- **Modules that import `__all__` from elsewhere**: `from ._impl import __all__` re-exports a list this module cannot
  see.

  ```python ok file=pkg/api.py
  from ._impl import __all__
  ```

  ```python context file=pkg/_impl.py
  LIMIT = 3
  __all__ = ["LIMIT"]
  ```

- **Names bound only under `if TYPE_CHECKING:`**: such a name exists for type checkers alone, so listing it makes
  `from module import *` raise [`AttributeError`](https://docs.python.org/3/builtins/exceptions.html#AttributeError) at
  run time.

  ```python ok file=pkg/typed.py
  from typing import TYPE_CHECKING

  if TYPE_CHECKING:

      class Locked(dict): ...


  def use(x: "Locked") -> None: ...


  __all__ = ["use"]
  ```

- **A name equal to the module's own name, when the package's `__init__.py` star-imports the module**: listing it would
  make the `import *` bind it over the package attribute that holds the submodule.

  ```python ok file=pkg/shared.py
  def shared() -> int:
      return 1


  __all__ = []
  ```

  ```python context file=pkg/__init__.py
  from .shared import *
  ```

  ```python context file=cli.py
  from pkg.shared import shared

  print(shared())
  ```

- **Names the module imports and uses itself**: another module reaching `sys` as `app.sys` borrows a name `app` needs,
  which is not part of its API.

  ```python ok file=app.py
  import sys

  print(sys.argv)
  ```

  ```python context file=cli.py
  import app

  print(app.sys)
  ```

- **Dotted strings and attribute patches**, such as `mocker.patch("pkg.mod.name")` and `patch.object(module, "name")`:
  they reach into the module without making the name part of its API. Fixes that rename or move a name still leave names
  reached this way alone.

  ```python ok file=config.py
  LIMIT = 3
  __all__ = []
  ```

  ```python context file=cli.py
  import config
  from unittest import mock

  with mock.patch("config.LIMIT"), mock.patch.object(config, "LIMIT"):
      pass
  ```

- **An `__all__` built any other way** than the forms listed under Reading `__all__` below, such as `list(...)` or
  `.remove(...)`, or bound inside a nested block such as an `if`, `try` or function: its contents depend on how the
  module runs, so nothing is reported.

  - a call, such as `list(...)`

    ```python ok file=config.py
    LIMIT = 3


    def local() -> int:
        return 1


    __all__ = list(["local"])
    ```

    ```python context file=cli.py
    from config import LIMIT

    print(LIMIT)
    ```

  - a binding in a nested block

    ```python ok file=config.py
    LIMIT = 3


    def local() -> int:
        return 1


    if LIMIT:
        __all__ = ["local"]
    ```

    ```python context file=cli.py
    from config import LIMIT

    print(LIMIT)
    ```

### Resolving modules

Module names resolve to files the way Ruff's import graph resolves them, from Ruff's
[`src`](https://docs.astral.sh/ruff/settings/#src) directories and the directory above each package. So a `src` layout
works, and `tests/helpers.py` imported as `tests.helpers` matches even when `tests` has no `__init__.py`.

A module of a namespace package, a directory without `__init__.py`, is named from the search root above it, as Python
names it, so its relative imports count.

### Reading `__all__`

simplint reads `__all__` from these forms, all holding string literals:

- module-level `=`, annotated and `+=` assignments of list or tuple literals
- `.extend([...])` and `.append("name")` calls

### Fix

What the fix does depends on whether the module already has an `__all__`, and on who star-imports it:

- **The module has an `__all__`**: the fix adds the names to it, and is safe.
- **It has none, and no other file may star-import it**: the fix creates one, and is unsafe.
- **It has none, and another file may star-import it**: no fix is offered.
- **A public module of a library**: no fix is offered at all, since listing a name widens the library's API and what
  `import *` binds; the author decides.

**Adding to an existing `__all__`**: the fix adds the missing names to the list or tuple literal of the last `=`,
annotated or `+=` assignment to `__all__`. Adding names is safe. No fix is offered when the empty list holds a comment,
since the rewrite would drop it. When a comment ends the last element's line, the new names go on the lines after it, so
the comment stays with its element; no fix is offered when that line holds other code, such as the closing bracket.

A `# noqa: F401` comment on an import is removed with its names only when every name the import binds on that line is
listed.

- Each name goes before the first listed name that Ruff's
  [unsorted-dunder-all](https://docs.astral.sh/ruff/rules/unsorted-dunder-all/) sorts after it, so a sorted `__all__`
  stays sorted.

  ```python report file=config.py
  LIMIT = 3


  def local() -> int:
      return 1


  def shared() -> int:
      return LIMIT + local()


  __all__ = [
      "LIMIT",
      "shared",
  ]
  ```

  ```python fixed file=config.py
  LIMIT = 3


  def local() -> int:
      return 1


  def shared() -> int:
      return LIMIT + local()


  __all__ = [
      "LIMIT",
      "local",
      "shared",
  ]
  ```

  ```python context file=cli.py
  from config import local

  print(local())
  ```

- A literal written on one line is rewritten one name per line with a trailing comma, the layout `ruff format` gives it.

  ```python report file=config.py
  LIMIT = 3


  def local() -> int:
      return 1


  __all__ = ["local"]
  ```

  ```python fixed file=config.py
  LIMIT = 3


  def local() -> int:
      return 1


  __all__ = [
      "LIMIT",
      "local",
  ]
  ```

  ```python context file=cli.py
  from config import LIMIT

  print(LIMIT)
  ```

- A parenthesized last name ends at its closing parenthesis: the new name goes after it, so the parentheses stay around
  the name.

  ```python report file=config.py
  LIMIT = 3


  def local() -> int:
      return 1


  __all__ = [  # the public names
      ("LIMIT"),
  ]
  ```

  ```python fixed file=config.py
  LIMIT = 3


  def local() -> int:
      return 1


  __all__ = [  # the public names
      ("LIMIT"),
      "local",
  ]
  ```

  ```python context file=cli.py
  from config import local

  print(local())
  ```

- An empty literal that holds a comment gets no fix, since the rewrite would drop the comment:

  ```python report file=config.py
  LIMIT = 3


  __all__ = [  # filled in by the release script
  ]
  ```

  ```python fixed file=config.py
  LIMIT = 3


  __all__ = [  # filled in by the release script
  ]
  ```

  ```python context file=cli.py
  from config import LIMIT

  print(LIMIT)
  ```

- A comment ending the last element's line stays with that element: the new names go on the lines after it.

  ```python report file=config.py
  LIMIT = 3


  def local() -> int:
      return 1


  __all__ = [
      "LIMIT",  # the limit
  ]
  ```

  ```python fixed file=config.py
  LIMIT = 3


  def local() -> int:
      return 1


  __all__ = [
      "LIMIT",  # the limit
      "local",
  ]
  ```

  ```python context file=cli.py
  from config import local

  print(local())
  ```

- A `# noqa: F401` on an import stays while a name the import binds is left out of the listing, since that name still
  needs it:

  ```python report file=config.py
  from math import pi, tau  # noqa: F401


  def area(radius: float) -> float:
      return 3.14 * radius**2


  __all__ = [
      "area",
  ]
  ```

  ```python fixed file=config.py
  from math import pi, tau  # noqa: F401


  def area(radius: float) -> float:
      return 3.14 * radius**2


  __all__ = [
      "area",
      "pi",
  ]
  ```

  ```python context file=cli.py
  from config import pi

  print(pi)
  ```

When several names are missing, only the first report carries the fix, since it adds all of them at once; the others
have none. A suppression comment on the first report moves the fix to the next report that is still shown, and the fix
still lists every name:

```python context file=use.py
from api import alpha, beta

print(alpha(), beta())
```

```python report file=api.py
__all__ = []


def alpha() -> int:  # noqa: SMP401
    return 1


def beta() -> int:
    return 2
```

```python fixed file=api.py
__all__ = [
    "alpha",
    "beta",
]


def alpha() -> int:  # noqa: SMP401
    return 1


def beta() -> int:
    return 2
```

**Creating a new `__all__`**: a module without `__all__` gets a new one, sorted and one name per line, as its last
statement, or above a closing `if __name__ == "__main__":` block. **Unsafe fix:** it narrows what `from module import *`
exports.

- Last statement of the module:

  ```python report file=config.py
  LIMIT = 3


  def local() -> int:
      return 1
  ```

  ```python fixed file=config.py
  LIMIT = 3


  def local() -> int:
      return 1


  __all__ = [
      "LIMIT",
  ]
  ```

  ```python context file=cli.py
  from config import LIMIT

  print(LIMIT)
  ```

- Above a closing `__main__` block:

  ```python report file=config.py
  LIMIT = 3


  def main() -> None:
      print(LIMIT)


  if __name__ == "__main__":
      main()
  ```

  ```python fixed file=config.py
  LIMIT = 3


  def main() -> None:
      print(LIMIT)


  __all__ = [
      "LIMIT",
  ]


  if __name__ == "__main__":
      main()
  ```

  ```python context file=cli.py
  from config import LIMIT

  print(LIMIT)
  ```

A module another file of the run star-imports, or may star-import, gets no new `__all__`. `import *` takes every public
name of it, and a name such as `ResourceType` that `pkg/__init__.py` re-exports with `from ._types import *` would
vanish from `pkg`. A name imported from `pkg` counts as imported from `pkg/_types.py`. Importers outside the run are not
seen.

```python report file=config.py
LIMIT = 3


def local() -> int:
    return 1
```

```python fixed file=config.py
LIMIT = 3


def local() -> int:
    return 1
```

```python context file=cli.py
from config import LIMIT

print(LIMIT)
```

```python context file=star.py
from config import *

print(local())
```

### Related rules

[unused-export](SMP402.html) removes the opposite: names `__all__` lists that no other module imports.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issues [#167](https://github.com/astral-sh/ruff/issues/167)
  (report importers of names `__all__` omits), [#2098](https://github.com/astral-sh/ruff/issues/2098) and
  [#3482](https://github.com/astral-sh/ruff/issues/3482) (modules with no `__all__`).
