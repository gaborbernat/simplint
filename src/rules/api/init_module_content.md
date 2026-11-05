Flags `__init__.py` files that hold anything besides a module docstring. Importing any submodule runs the package's
`__init__.py` first, so code there slows down or breaks unrelated modules.

## Why is this bad?

Python runs a package's `__init__.py` before it imports any of the package's submodules. Importing `pkg.sub` therefore
also runs everything in `pkg/__init__.py`, and a slow or failing import there slows down or breaks unrelated modules.
Re-exports give one object two import paths and invite circular imports.

## Example

```python context file=http_client/client.py
class Client:
    pass
```

```python report file=http_client/__init__.py
"""HTTP client."""

from .client import Client

__all__ = ["Client"]
```

Use instead:

```python context file=http_client/client.py
class Client:
    pass
```

```python ok file=http_client/__init__.py
"""HTTP client."""
```

Importers then use `from http_client.client import Client`. In a library whose public package presents the API, keep the
code in a private module and only re-export it:

```toml config
[tool.simplint]
project-kind = "library"
```

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
  [`__init__.py`](https://docs.python.org/3/reference/import.html#regular-packages)**: every one is reported, including
  imports and [`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package), since each runs
  whenever any submodule is imported.

  ```python report file=pkg/__init__.py
  """HTTP client."""

  from .client import Client

  __all__ = ["Client"]
  ```

- **Other statements in a public package of a library**: anything beyond the re-exports listed under Exceptions is
  reported there too.

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
    ```

  - any other statement, such as an assignment:

    ```toml config
    [build-system]
    requires = ["setuptools"]
    build-backend = "setuptools.build_meta"

    [project]
    name = "demo"
    ```

    ```python report file=demo/__init__.py
    """HTTP client."""

    VERSION = "1.0"
    ```

### Exceptions

- **The module docstring**: it documents the package and runs no code.

  ```python ok file=pkg/__init__.py
  """HTTP client."""
  ```

- **Re-exports in a public package of a library**: such a package may re-export its public API, as
  [PEP 8](https://peps.python.org/pep-0008/#public-and-internal-interfaces) describes for a package's `__init__` that
  exposes functionality from submodules. Its `__init__.py` may hold:

  - imports whose every name it exports through `__all__`:

    ```toml config
    [build-system]
    requires = ["setuptools"]
    build-backend = "setuptools.build_meta"

    [project]
    name = "demo"
    ```

    ```python ok file=demo/__init__.py
    """HTTP client."""

    from ._core import run

    __all__ = ["run"]
    ```

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

  - the `__all__` that lists them

    ```toml config
    [build-system]
    requires = ["setuptools"]
    build-backend = "setuptools.build_meta"

    [project]
    name = "demo"
    ```

    ```python ok file=demo/__init__.py
    """HTTP client."""

    __all__ = ["run"]
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

### Public packages

A package is **public** when it belongs to one of the library's API packages and no part of its path starts with `_`;
see [undocumented-public-api](SMP903.html).
