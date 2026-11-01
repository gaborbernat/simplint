## What it does

Checks for `Callable[..., T]`: a subscript whose first argument is `...` and whose name, or the last part of a dotted
name, is `Callable`. That covers [`typing.Callable`](https://docs.python.org/3/library/typing.html#typing.Callable),
[`collections.abc.Callable`](https://docs.python.org/3/library/collections.abc.html#collections.abc.Callable) and a bare
`Callable` imported from either. The check goes by name only, so it also matches type aliases and other expressions
outside annotations. String annotations are not checked.

## Why is this bad?

`...` accepts any arguments, so the type checker cannot catch a call with the wrong number, type or names of arguments.
A [`Protocol`](https://docs.python.org/3/library/typing.html#typing.Protocol)
([PEP 544](https://peps.python.org/pep-0544/)) with a `__call__` method, called a
[callback protocol](https://typing.python.org/en/latest/spec/callables.html#callback-protocols), states the parameters.
It can also state keyword names and defaults, which a parameter list such as `Callable[[str, int], T]` cannot express.

## Example

```python
def run(callback: Callable[..., None]) -> None:
    callback("build", retries=3)
```

Use instead:

```python
class Callback(Protocol):
    def __call__(self, name: str, *, retries: int) -> None: ...


def run(callback: Callback) -> None:
    callback("build", retries=3)
```

## Fix safety

No fix is offered: the real signature has to be written by hand.
