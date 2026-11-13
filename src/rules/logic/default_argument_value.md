Flags arguments that pass a parameter of a function in the same module the literal its default already gives.

## Why is this bad?

An argument equal to the default tells the reader the value matters at this call when it does not. It also pins the call
to today's default: when the function's author changes the default, this call keeps the old value, which may not be what
its author meant. Leaving the argument out says the call wants whatever the function does by default.

## Example

```python report
def fetch(url: str, retries: int = 3) -> str:
    return f"{url} {retries}"


print(fetch("a"), fetch("b", retries=3))
```

Use instead:

```python ok
def fetch(url: str, retries: int = 3) -> str:
    return f"{url} {retries}"


print(fetch("a"), fetch("b"))
```

## What it does

The rule reads the calls a module makes by name to its own module-level functions and compares each argument to the
default of the parameter it fills.

### Active on

- **Keyword arguments** equal to the default.

- **The last positional argument**, when it equals the default of its parameter.

  ```python report
  def fetch(url: str, retries: int = 3) -> str:
      return f"{url} {retries}"


  print(fetch("a"), fetch("b", 3))
  ```

### Exceptions

- **Defaults that are not literals**: a default such as `[]`, `CONFIG` or `time.time()` may differ from the value the
  call passes even when both are spelled alike, so only `None`, booleans, numbers, strings, bytes and `...` count.

- **Earlier positional arguments**: leaving one out would shift the arguments after it onto other parameters.

  ```python ok
  def scale(value: int, factor: int = 1, offset: int = 0) -> int:
      return value * factor + offset


  print(scale(2), scale(2, 1, 5))
  ```

- **Parameters every call passes**: [`unused-default`](SMP506.html) reports their default instead, since the
  default is then never used.

  ```python ok
  def _scale(value: int, factor: int = 1) -> int:
      return value * factor


  print(_scale(2, 1), _scale(3, 2))
  ```

- **Decorated or rebound functions**: a decorator may change the signature a call reaches, and a rebound name may reach
  another function.

- **Calls that unpack `*args` or `**kwargs`**, and calls of functions in other modules.

### Fix

The fix removes the argument with the comma that separates it. The literal default gives the parameter an equal value,
so the fix is safe. An argument with a comment in the call gets no fix.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** no rule for functions of the project; rules such as
  [UP015](https://docs.astral.sh/ruff/rules/redundant-open-modes/) cover the defaults of specific standard library
  calls.
