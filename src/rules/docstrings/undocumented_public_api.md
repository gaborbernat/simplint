## What it does

Checks that a library's public API is documented. The public API is what users of the library can import, following the
[typing spec](https://typing.python.org/en/latest/spec/distributing.html#library-interface-public-and-private-symbols)
and [PEP 8](https://peps.python.org/pep-0008/#public-and-internal-interfaces):

- a module is public when it belongs to one of the library's API packages and no part of its path starts with `_`, so
  everything under `package/_internal/` or in `package/_core.py` is internal;
- a public module offers the names its
  [`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package) lists, or, without `__all__`,
  every name it defines without a leading `_`, and the names another module imports from it, which [SMP113](SMP113.html)
  adds to `__all__`;
- a name a public module re-exports, by listing it in `__all__`, importing it as `from ._core import name as name`, or
  through `from ._core import *`, is public where it is defined, through any chain of re-exports. A plain
  `from ._core import name` in a module without `__all__` keeps the name internal.

What it asks for:

- a public function or class needs a [docstring](https://docs.python.org/3/glossary.html#term-docstring), and so do the
  methods, properties and nested classes of a public class whose names do not start with `_`;
- a public variable needs a docstring on the line below it, or a `#:` comment on the line above it or at the end of its
  line, the forms
  [Sphinx autodoc](https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html#directive-autoattribute) reads.

These are skipped:

- functions decorated with [`@overload`](https://docs.python.org/3/library/typing.html#typing.overload), since the
  implementation carries the docstring
- dunder methods, such as `__init__`, whose parameters the class docstring describes
- methods decorated with [`@override`](https://docs.python.org/3/library/typing.html#typing.override), since
  documentation tools show the base method's docstring
- property setters and deleters, which share the property's docstring
- test modules, as defined in [SMP200](SMP200.html)
- every module of an application, which has no Python API

A module that builds `__all__` dynamically, such as with `list(...)`, or supplies names through a module-level
`__getattr__`, is checked only for the names other modules re-export, because its own exports cannot be read.

The report names the module that re-exports the name when its own module is private, so you can see why it counts.

## Why is this bad?

Users of a library reach its public API through their editor's hover help and the generated reference, not through the
source. A docstring is what they read there: what the function promises, what each parameter means, what it returns and
what it raises. Internal code is the opposite case: its readers have the source open, so [SMP002](SMP002.html) asks for
no docstring there.

## Example

```python
# package/__init__.py
from ._client import fetch

__all__ = ["fetch"]


# package/_client.py
def fetch(url: str) -> bytes:
    return _download(url)
```

Use instead:

```python
# package/_client.py
def fetch(url: str) -> bytes:
    """Download the resource at a URL.

    :param url: Address of the resource.
    :returns: The response body.
    """
    return _download(url)
```

## Fix safety

No fix is offered, since only the author knows what the API promises.

## Options

- `project-kind` in `[tool.simplint]`: whether the project is a library or an application
- `api-packages` in `[tool.simplint]`: the top-level packages whose modules make up the public API
