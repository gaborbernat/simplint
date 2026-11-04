## What it does

Checks reStructuredText docstrings, when `docstring-style = "rest"`, for `:param:` and `:ivar:`, `:cvar:` or `:var:`
fields whose description only repeats the name: every word it has, beyond articles and words such as "parameter" or
"value", is a word of the name, as `:param user_id: The user ID.`.

## Why is this bad?

Such a description takes the space of a useful one while telling readers nothing the signature does not: not the unit,
the accepted range, what `None` means, or how the value is used. It is often what fills a field added only to satisfy a
check.

## Example

```python
def fetch(url: str, timeout: float) -> bytes:
    """Download a resource.

    :param url: The URL.
    :param timeout: The timeout.
    """
```

Use instead:

```python
def fetch(url: str, timeout: float) -> bytes:
    """Download a resource.

    :param url: Address of the resource, with its scheme.
    :param timeout: Seconds to wait for the first byte.
    """
```

## Fix safety

No fix is offered, since only the author knows what the value means.

## Options

- `docstring-style` in `[tool.simplint]`: `"rest"` turns the rule on
