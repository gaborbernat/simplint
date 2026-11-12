Flags a value assigned to a local name that a later assignment in the same block replaces before anything reads it.

## Why is this bad?

The first value is never used, so the line that computes it does nothing for the function. A reader takes it for a
starting value that matters, and has to scan down to the second assignment to learn that it does not. Often the read it
was written for got lost in a change, which hides a bug.

## Example

```python report
def size(items: list[int]) -> int:
    total = 0
    total = len(items)
    return total
```

```python fixed
def size(items: list[int]) -> int:
    total = len(items)
    return total
```

## What it does

### Active on

An assignment of a value to one local name, plain or annotated, inside a function, when the next statement of the same
block that mentions the name assigns it again without reading it.

- **A value that calls something**: the call still runs, so the assignment is reported without a fix.

  ```python report
  def latest(items: list[int]) -> int:
      found = max(items)
      found = items[-1]
      return found
  ```

### Exceptions

- **A block inside a `try` or a `with`**: an exception between the two assignments may go on to a handler, a `finally`
  or past the `with`, where the first value is read.

  ```python ok
  def size(items: list[int]) -> int:
      try:
          total = 0
          total = len(items)
      finally:
          print("done")
      return total
  ```

- **A `break` or `continue` between the two**: the loop may read the first value before the second assignment.

- **A name a nested function, class or lambda mentions, or one `global` or `nonlocal` declares**: it may be read at any
  time.

  ```python ok
  from collections.abc import Callable


  def counter() -> Callable[[], int]:
      count = 0

      def read() -> int:
          return count

      count = 1
      return read
  ```

- **A function that reads its locals by name**, through `locals()`, `vars()`, `eval` or `exec`.

- **`None` assigned over a computed value**: it lets the value go before the function ends, often to free its memory.

  ```python ok
  def release(size: int) -> None:
      buffer = bytearray(size)
      buffer = None
      print(buffer)
  ```

- **An assignment in the next statement that reads the name**, such as `total = total + 1`.

### Fix

The fix removes an assignment of a constant, and leaves an annotated one as a bare annotation, when it sits on lines of
its own with no comment. Building a constant does nothing else, so the fix is safe. Any other value is left in place.

```python report
def size(items: list[int]) -> int:
    total: int = 0
    total = len(items)
    return total
```

```python fixed
def size(items: list[int]) -> int:
    total: int
    total = len(items)
    return total
```

## Ruff overlap

Ruff's [unused-variable](https://docs.astral.sh/ruff/rules/unused-variable/) (F841) reports a name that is never read at
all, and [unnecessary-assign](https://docs.astral.sh/ruff/rules/unnecessary-assign/) (RET504) an assignment right before
a `return` of the name. Neither reports a value that a later assignment replaces.

## Other tools

- [refurb](https://github.com/dosisod/refurb) and [semgrep](https://github.com/semgrep/semgrep-rules)'s
  `useless-assignment` report some of these, and PyCharm reports them as a redundant assignment.
