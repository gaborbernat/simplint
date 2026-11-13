Flags [`functools.cache`](https://docs.python.org/3/library/functools.html#functools.cache),
[`functools.lru_cache`](https://docs.python.org/3/library/functools.html#functools.lru_cache) or
[`functools.cached_property`](https://docs.python.org/3/library/functools.html#functools.cached_property) on a function
that yields.

## Why is this bad?

Calling a generator function returns a generator object, and the cache stores that object. The first caller iterates it
to the end; every later call with the same arguments gets the same exhausted object back, which yields nothing. The code
looks like it caches the values, and the second call silently returns an empty result.

## Example

```python report
import functools


@functools.cache
def squares(limit: int):
    for number in range(limit):
        yield number * number
```

Use instead:

```python ok
import functools


@functools.cache
def squares(limit: int) -> tuple[int, ...]:
    return tuple(number * number for number in range(limit))
```

## What it does

### Active on

- **Functions and static or class methods** decorated with `cache` or `lru_cache`, called or bare, spelled through
  `functools`, an alias or a `from functools import`. An `async def` that yields counts too.

- **Methods decorated with `cached_property`**, whose generator the instance keeps after the first access.

  ```python report
  from functools import cached_property


  class Report:
      @cached_property
      def lines(self):
          yield "header"
  ```

### Exceptions

- **`cache` and `lru_cache` on an instance method** are left to Ruff, see below.

## Ruff overlap

Ruff's [cached-instance-method (B019)](https://docs.astral.sh/ruff/rules/cached-instance-method/) reports `cache` and
`lru_cache` on any instance method, generator or not, because the cache keeps `self` alive. This rule skips those and
reports the rest, which Ruff does not.

## Other tools

No Ruff, Pylint or flake8 rule reports a cached generator.
