Flags `__init__.py` files that hold code beyond a module docstring and deliberate API re-exports.

## Why is this bad?

The first import of `pkg.sub`
[runs `pkg/__init__.py` first](https://docs.python.org/3/reference/import.html#regular-packages). An expensive import
there adds to the startup cost even for callers that only need one submodule.
[Scientific Python's SPEC 1](https://scientific-python.org/specs/spec-0001/) describes this cost and how lazy loading
reduces it.

Unplanned re-exports give one object two import paths and invite circular imports.
[PEP 8](https://peps.python.org/pep-0008/#public-and-internal-interfaces) treats imported names as internal unless a
package's `__init__` documents them as its API, so a package can still expose selected names from private modules
through `__all__`.

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

Moving code out of `__init__.py` can change its import path or when it runs. The rule offers no fix.

### Active on

- **Imports used in the initializer** can expose names unintentionally. In the example below, `Client` is imported to
  construct `DEFAULT`. simplint reports the import and leaves the assignment to Ruff's RUF067.

  ```python report file=pkg/__init__.py
  """HTTP client."""

  from .client import Client

  DEFAULT = Client()
  ```

- **Unexported imports and dunder assignments** can also trigger this rule in a public library package.

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

- **The module docstring** documents the package.

  ```python ok file=pkg/__init__.py
  """HTTP client."""
  ```

- **Names re-exported with `from ... import ...`** are allowed in any package when its own, statically readable
  `__all__` lists them.

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

- **[`from __future__` imports](https://docs.python.org/3/reference/simple_stmts.html#future-statements)** select
  language features for the current module.

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

- **[`__lazy_modules__`](https://peps.python.org/pep-0810/#what-about-forwards-compatibility-with-older-python-versions)**
  marks imports in the initializer as eligible for lazy loading on Python 3.15 and later.

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

- **A `__version__` that build or release tooling reads**: the assignment has to stay where the tool looks. That is the
  file setuptools' `attr:`, hatch's or pdm's version `path`, flit, bump-my-version, bump2version or tbump name, and any
  package's `__init__.py` when `setup.py` reads `__version__` itself.

  ```toml config
  [build-system]
  requires = ["hatchling"]
  build-backend = "hatchling.build"

  [project]
  name = "demo"
  dynamic = ["version"]

  [tool.hatch.version]
  path = "demo/__init__.py"
  ```

  ```python ok file=demo/__init__.py
  """HTTP client."""

  __version__ = "1.0"
  ```

- **Unused imports**: [F401](https://docs.astral.sh/ruff/rules/unused-import/) reports an import that nothing reads, so
  this rule does not report it, whether or not the run selects F401. A statement keeps its report when one of its names
  is not one F401 reports, such as a name that a string Ruff reads as a type expression spells, a star import, or a
  redundant alias that the package does not expose.

  ```python ok file=pkg/__init__.py
  """HTTP client."""

  from .client import Client
  ```

- **Statements covered by [RUF067](https://docs.astral.sh/ruff/rules/non-empty-init-module/)** include class and
  function definitions and ordinary assignments. simplint leaves them to Ruff even when RUF067 is disabled. Ruff's
  `strictly-empty-init-modules` setting extends that check to imports and dunder assignments. simplint can still report
  an attribute docstring that follows a simple assignment.

  ```python ok file=pkg/__init__.py
  """HTTP client."""

  VERSION = "1.0"
  ```

### Public packages

A package is **public** when it belongs to one of the library's API packages and no part of its path starts with `_`;
see [undocumented-public-api](SMP903.html).

## Other tools

- Ruff's [RUF067](https://docs.astral.sh/ruff/rules/non-empty-init-module/) checks for code in initializers. Its default
  settings allow re-exports. simplint also checks which names a package re-exports.
- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports logic in an `__init__`
  module as
  [WPS412](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/best_practices.html#wemake_python_styleguide.violations.best_practices.InitModuleHasLogicViolation),
  with no fix.
