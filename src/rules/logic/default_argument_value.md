Flags a call that explicitly passes a parameter's literal default to a function defined in the same module.

## Why is this bad?

Omitting an argument lets the function choose its default. An explicit `retries=3` keeps the call at three retries even
if the default changes. Keep the argument when that distinction matters to the caller.

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

- **Defaults outside the supported scalar literals.** A default such as `[]`, `CONFIG` or
  [`time.time()`](https://docs.python.org/3/library/time.html#time.time) may differ from the value the call passes even
  when both are spelled alike, so only `None`, booleans, numbers, strings, bytes and `...` count.

- **Earlier positional arguments**: leaving one out would shift the arguments after it onto other parameters.

  ```python ok
  def scale(value: int, factor: int = 1, offset: int = 0) -> int:
      return value * factor + offset


  print(scale(2), scale(2, 1, 5))
  ```

- **Parameters that every known call supplies.** When simplint can account for callers of a non-public function, it
  leaves these parameters to [`unused-default`](SMP506.html).

  ```python ok
  def _scale(value: int, factor: int = 1) -> int:
      return value * factor


  print(_scale(2, 1), _scale(3, 2))
  ```

- **Dunder names, rebound names and most decorators.** A call may reach a different function or signature. The rule
  recognizes `classmethod`, [`abc.abstractmethod`](https://docs.python.org/3/library/abc.html#abc.abstractmethod),
  [`functools.wraps`](https://docs.python.org/3/library/functools.html#functools.wraps),
  [`contextlib.contextmanager`](https://docs.python.org/3/library/contextlib.html#contextlib.contextmanager),
  [`contextlib.asynccontextmanager`](https://docs.python.org/3/library/contextlib.html#contextlib.asynccontextmanager),
  and the `final` and `override` decorators from `typing` and `typing_extensions`.

- **Cached functions**: [`functools.cache`](https://docs.python.org/3/library/functools.html#functools.cache) and
  [`functools.lru_cache`](https://docs.python.org/3/library/functools.html#functools.lru_cache) can store separate
  results for an explicit default and an omitted argument.

- **Calls that unpack `*args` or `**kwargs`**, and calls of functions in other modules.

### Fix

The fix removes the argument with the comma that separates it. The literal default gives the parameter an equal value,
so the fix is safe. A comment anywhere in the call prevents the fix.

## Other tools

- Ruff's [UP015](https://docs.astral.sh/ruff/rules/redundant-open-modes/) removes redundant mode arguments from
  [`open()`](https://docs.python.org/3/builtins/functions.html#open).
