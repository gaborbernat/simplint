## What it does

Checks for [`__init__.py`](https://docs.python.org/3/reference/import.html#regular-packages) files that hold anything
besides a module docstring. Every statement after the docstring is reported, including imports and
[`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package).

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

Importers then use `from http_client.client import Client`.

## Fix safety

No fix is offered: removing the code breaks every importer that relies on it, and moving it needs a decision about where
it belongs.
