Flags [`functools.cache`](https://docs.python.org/3/library/functools.html#functools.cache),
[`functools.lru_cache`](https://docs.python.org/3/library/functools.html#functools.lru_cache) or
[`functools.cached_property`](https://docs.python.org/3/library/functools.html#functools.cached_property) on a function
that yields.

## Why is this bad?

A cache hit returns the same generator at its current position. If one caller consumes a few values, the next caller
starts partway through. Once a caller exhausts the generator, further hits yield nothing. Cache a tuple when callers
need to iterate over the values independently.

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

- **Functions, static methods and class methods** decorated with `cache` or `lru_cache`, including `@lru_cache(...)`.
  The rule resolves aliases and direct imports from `functools`. It also checks async generator functions.

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

Ruff's [cached-instance-method (B019)](https://docs.astral.sh/ruff/rules/cached-instance-method/) checks instance
methods decorated with `cache` or `lru_cache`. The cache can keep `self` alive because the instance is part of the cache
key. simplint skips those decorators on instance methods.
