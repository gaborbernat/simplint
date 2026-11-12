Flags `__init__.py` files that hold code beyond a module docstring and deliberate API re-exports.

## Why is this bad?

Python runs a package's `__init__.py` before it imports any of the package's submodules. Importing `pkg.sub` therefore
also runs everything in `pkg/__init__.py`, and a slow or failing import there slows down or breaks unrelated modules.
Unplanned re-exports give one object two import paths and invite circular imports. A package can deliberately expose
selected names from private modules through `__all__`.

## Example

```python context file=http_client/client.py
class Client:
    pass
```

```python report file=http_client/__init__.py
"""HTTP client."""

from .client import Client

DEFAULT = Client()
```

Use instead:

```python context file=http_client/client.py
class Client:
    pass
```

```python ok file=http_client/__init__.py
"""HTTP client."""
```

Importers then use `from http_client.client import Client`. When the package presents an API, keep the implementation in
a private module and list the names it exposes:

```python context file=http_client/_client.py
class Client:
    pass
```

```python ok file=http_client/__init__.py
"""HTTP client."""

from ._client import Client

__all__ = ["Client"]
```

## What it does

No fix is offered: removing the code breaks every importer that relies on it, and moving it needs a decision about where
it belongs.

### Active on

- **Statements after the docstring of an
  [`__init__.py`](https://docs.python.org/3/reference/import.html#regular-packages)** that do not form an explicit API:
  imports absent from [`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package) and direct
  definitions are reported.

  ```python report file=pkg/__init__.py
  """HTTP client."""

  from .client import Client

  DEFAULT = Client()
  ```

- **Other statements in a public package of a library**: an import beyond the re-exports listed under Exceptions, and a
  dunder assignment, are reported there too. The other statements are left to RUF067, see Exceptions.

  - an import of a name it does not export:

    ```toml config
    [build-system]
    requires = ["setuptools"]
    build-backend = "setuptools.build_meta"

    [project]
    name = "demo"
    ```

    ```python report file=demo/__init__.py
    """HTTP client."""

    from ._core import run
    from ._core import stop as stop

    __all__ = ["stop"]

    DEFAULT = run
    ```

  - a dunder assignment, such as `__version__`, which Ruff's RUF067 allows:

    ```toml config
    [build-system]
    requires = ["setuptools"]
    build-backend = "setuptools.build_meta"

    [project]
    name = "demo"
    ```

    ```python report file=demo/__init__.py
    """HTTP client."""

    __version__ = "1.0"
    ```

### Exceptions

- **Test packages and nested test projects**: fixtures may need initializer code to exercise import behavior. A package
  under a test root that is itself a project is treated as test input, even when its module name does not start with
  `test_`.

  ```python context file=test-root/setup.py
  from setuptools import setup

  setup(name="sample")
  ```

  ```python ok file=test-root/pkg/__init__.py
  """Fixture package for import tests."""

  from ._core import run

  run()
  ```

- **PEP 562 package hooks and `if TYPE_CHECKING` blocks**: `__getattr__` can load package attributes on demand,
  `__dir__` can list them, and a `TYPE_CHECKING` block can expose their types to type checkers. The imports used by
  those hooks may still need a suppression when they are not re-exports.

  ```python ok file=pkg/__init__.py
  """HTTP client."""


  def __getattr__(name: str) -> object:
      from importlib import import_module

      return getattr(import_module("pkg._client"), name)
  ```

- **The module docstring**: it documents the package and runs no code.

  ```python ok file=pkg/__init__.py
  """HTTP client."""
  ```

- **Names listed in `__all__`**: any package can expose selected names from a private module. The list must be readable
  statically; an unlisted import is still reported. This works in applications and private subpackages as well as public
  library packages.

  ```python context file=pkg/_api.py
  class API:
      pass
  ```

  ```python ok file=pkg/__init__.py
  from ._api import API

  __all__ = ["API"]
  ```

- **Other re-exports in a public package of a library**:
  [PEP 8](https://peps.python.org/pep-0008/#public-and-internal-interfaces) describes a package's `__init__` exposing
  functionality from submodules. A public library package may also hold:

  - a redundant alias, such as `from ._core import run as run`:

    ```toml config
    [build-system]
    requires = ["setuptools"]
    build-backend = "setuptools.build_meta"

    [project]
    name = "demo"
    ```

    ```python ok file=demo/__init__.py
    """HTTP client."""

    from ._core import run as run
    ```

  - `import *`:

    ```toml config
    [build-system]
    requires = ["setuptools"]
    build-backend = "setuptools.build_meta"

    [project]
    name = "demo"
    ```

    ```python ok file=demo/__init__.py
    """HTTP client."""

    from ._core import *
    ```

  - `from __future__` imports, which change no behavior at run time

    ```toml config
    [build-system]
    requires = ["setuptools"]
    build-backend = "setuptools.build_meta"

    [project]
    name = "demo"
    ```

    ```python ok file=demo/__init__.py
    """HTTP client."""

    from __future__ import annotations
    ```

  - `__lazy_modules__`, the modules Python 3.15 loads lazily when the package's imports run

    ```toml config
    [build-system]
    requires = ["setuptools"]
    build-backend = "setuptools.build_meta"

    [project]
    name = "demo"
    ```

    ```python ok file=demo/__init__.py
    """HTTP client."""

    __lazy_modules__ = ["demo._core"]

    from ._core import run

    __all__ = ["run"]
    ```

- **Unused imports**: [F401](https://docs.astral.sh/ruff/rules/unused-import/) reports an import that nothing reads, so
  this rule does not report it, whether or not the run selects F401. A statement keeps its report when one of its names
  is not one F401 reports, such as a name that a string Ruff reads as a type expression spells, a star import, or a
  redundant alias that the package does not expose.

  ```python ok file=pkg/__init__.py
  """HTTP client."""

  from .client import Client
  ```

- **Statements Ruff reports**: [RUF067](https://docs.astral.sh/ruff/rules/non-empty-init-module/) reports the statements
  of an `__init__.py` that it does not allow, so this rule does not report them, whether or not the run selects RUF067
  or Ruff's preview mode is on. Ruff allows imports, dunder assignments and `if TYPE_CHECKING` blocks, unless
  `strictly-empty-init-modules` is set.

  ```python ok file=pkg/__init__.py
  """HTTP client."""

  VERSION = "1.0"
  ```

### Public packages

A package is **public** when it belongs to one of the library's API packages and no part of its path starts with `_`;
see [undocumented-public-api](SMP903.html).

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of rule
  [RUF067](https://docs.astral.sh/ruff/rules/non-empty-init-module/), which reports code in `__init__.py` except imports
  and dunder assignments (all of it with `strictly-empty-init-modules`) and which this rule leaves to it; this rule also
  reports the imports a library's public package does not re-export.
- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports logic in an `__init__`
  module as
  [WPS412](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/best_practices.html#wemake_python_styleguide.violations.best_practices.InitModuleHasLogicViolation),
  with no fix.
