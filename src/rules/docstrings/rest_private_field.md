## What it does

Checks the docstrings of a library's public API (see [SMP302](SMP302.html)), when `docstring-style = "rest"`, for
`:param:`, `:type:`, `:ivar:`, `:cvar:`, `:var:` and `:vartype:` fields naming a parameter or attribute with a leading
underscore.

## Why is this bad?

A leading underscore marks a name as internal ([PEP 8](https://peps.python.org/pep-0008/#public-and-internal-interfaces)),
so documenting it in the public reference invites users to depend on it. [SMP312](SMP312.html) does not ask for such
parameters, and [SMP317](SMP317.html) does not ask for such attributes.

## Example

```python
def fetch(url: str, _retries: int = 3) -> bytes:
    """Download a resource.

    :param url: Address of the resource.
    :param _retries: How often to retry, for the test suite.
    """
```

Use instead:

```python
def fetch(url: str, _retries: int = 3) -> bytes:
    """Download a resource.

    :param url: Address of the resource.
    """
```

## Fix safety

The fix deletes the field with the lines that continue it. It is unsafe, since code may read the docstring through
`__doc__`. No fix is offered when escapes in the docstring make its text differ from the source.

## Options

- `docstring-style` in `[tool.simplint]`: `"rest"` turns the rule on
