## What it does

Checks that a module's public API is documented. The public API is every name the module lists in
[`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package) or that another module in the run
imports (which [SMP113](SMP113.html) adds to `__all__`), plus the public methods of an exported class:

- an exported function or class needs a [docstring](https://docs.python.org/3/glossary.html#term-docstring);
- a method of an exported class whose name does not start with `_` needs a docstring;
- an exported variable needs a docstring on the line below it, or a `#:` comment on the line above it or at the end of
  its line, the forms
  [Sphinx autodoc](https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html#directive-autoattribute) reads.

These are skipped:

- functions decorated with [`@overload`](https://docs.python.org/3/library/typing.html#typing.overload), since the
  implementation carries the docstring
- dunder methods, such as `__init__`
- methods decorated with [`@override`](https://docs.python.org/3/library/typing.html#typing.override), since
  documentation tools show the base method's docstring
- property setters and deleters, which share the property's docstring
- test modules, as defined in [SMP200](SMP200.html)

In a module without `__all__`, only the names other modules import count as exported. A module that builds `__all__`
dynamically, such as with `list(...)`, is checked only for those imported names, because its own exports cannot be read.

## Why is this bad?

Users of a library reach its public API through their editor's hover help and the generated reference, not through the
source. A docstring is what they read there: what the function promises, what each parameter means, what it returns and
what it raises. Internal code is the opposite case: its readers have the source open, so [SMP002](SMP002.html) asks for
no docstring there. Listing a name in `__all__` is what moves it from one side to the other.

## Example

```python
__all__ = ["DEFAULT_TIMEOUT", "fetch"]

DEFAULT_TIMEOUT = 30


def fetch(url: str, timeout: float = DEFAULT_TIMEOUT) -> bytes:
    return _download(url, timeout)
```

Use instead:

```python
__all__ = ["DEFAULT_TIMEOUT", "fetch"]

#: Seconds to wait for a response before giving up.
DEFAULT_TIMEOUT = 30


def fetch(url: str, timeout: float = DEFAULT_TIMEOUT) -> bytes:
    """Download the resource at a URL.

    :param url: Address of the resource.
    :param timeout: Seconds to wait for a response.
    :returns: The response body.
    :raises TimeoutError: When the server does not answer in time.
    """
    return _download(url, timeout)
```

## Fix safety

No fix is offered, since only the author knows what the API promises.
