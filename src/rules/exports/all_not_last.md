## What it does

Checks for modules where the last top-level statement that binds
[`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package) is not the last statement. An
assignment, an annotated assignment or an augmented assignment such as `+=` binds it; a call such as
`__all__.extend(...)` does not. A trailing
[`if __name__ == "__main__":`](https://docs.python.org/3/library/__main__.html#idiomatic-usage) block may follow it,
also when written as `"__main__" == __name__`. Any other trailing `if`, such as `if __name__ != "__main__":`, counts as
a regular statement.

## Why is this bad?

Readers look for the export list in one place. At the end of the module it comes after every definition it names, so it
reads as a summary of what the module provides. [SMP113](SMP113.html) and [SMP115](SMP115.html) also check `__all__`.

## Example

```python
__all__ = ["main"]


def main() -> None:
    run()


if __name__ == "__main__":
    main()
```

Use instead:

```python
def main() -> None:
    run()


__all__ = ["main"]


if __name__ == "__main__":
    main()
```

## Fix safety

The fix is safe. It moves the statement to the end of the module, before a trailing `__main__` block, with two blank
lines before it, also when the file has no trailing newline. Comments on the lines above the statement stay where they
are.

The fix is offered only when a single statement binds `__all__`, no later statement reads `__all__`, and the statement
shares no line with other code. With several bindings or a later reader, moving the statement could change the value of
`__all__` or what that reader sees.
