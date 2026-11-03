## What it does

Checks for [`__init__.py`](https://docs.python.org/3/reference/import.html#regular-packages) files that hold anything
besides a module docstring. Every statement after the docstring is reported, including imports and
[`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package).

A public package of a library may also re-export its public API, as
[PEP 8](https://peps.python.org/pep-0008/#public-and-internal-interfaces) describes for a package's `__init__` that
exposes functionality from submodules: its `__init__.py` may hold imports whose every name it exports, through
`__all__`, a redundant alias such as `from ._core import run as run`, or `import *`, the `__all__` that lists them, and
`from __future__` imports. Any other statement is reported there too, including an import of a name it does not export.
A package is public when it belongs to one of the library's API packages and no part of its path starts with `_`; see
[SMP116](SMP116.html).

## Why is this bad?

Python runs a package's `__init__.py` before it imports any of the package's submodules. Importing `pkg.sub` therefore
also runs everything in `pkg/__init__.py`, and a slow or failing import there slows down or breaks unrelated modules.
Re-exports give one object two import paths and invite circular imports.

## Example

```python
"""HTTP client."""

from .client import Client

__all__ = ["Client"]
```

Use instead:

```python
"""HTTP client."""
```

Importers then use `from http_client.client import Client`. In a library whose public package presents the API, keep the
code in a private module and only re-export it:

```python
"""HTTP client."""

from ._client import Client

__all__ = ["Client"]
```

## Fix safety

No fix is offered: removing the code breaks every importer that relies on it, and moving it needs a decision about where
it belongs.
