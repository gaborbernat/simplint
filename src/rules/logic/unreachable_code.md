Flags statements that never run: those after a `return`, `raise`, `break` or `continue` in the same block, after a
statement every branch of which leaves the block, and the body of an `if` or `while` whose test is `False` or `0`.

## Why is this bad?

A statement after an unconditional return can make a function appear to perform work it always skips. Removing the
statement or moving it before the return makes the intended behavior clear.

## Example

```python report
def parse(text: str) -> int:
    return int(text)
    print("parsed")
```

```python fixed
def parse(text: str) -> int:
    return int(text)
```

## What it does

### Active on

- **Statements after one that always leaves the block**: a `return`, `raise`, `break` or `continue`, an `if` whose
  branches, `else` included, all leave, a `try` whose `finally` always leaves, or one whose body or `else` and every
  handler leave. A `while True` loop without a `break` also counts, as does an integer literal from 1 to 255 in place of
  `True`.

  ```python report
  def sign(value: int) -> int:
      if value < 0:
          return -1
      else:
          return 1
      print(value)
  ```

  ```python report
  def serve(queue: list[str]) -> None:
      while True:
          queue.pop()
      print("done")
  ```

- **The body of an `if` or `while` whose test is `False` or `0`**: a switch that turns code off.

  ```python report
  def check(value: int) -> int:
      if False:
          print(value)
      return value
  ```

### Exceptions

- **A `with` block**: its context manager may swallow an exception, even one raised while a `return` builds its value,
  and the code after the `with` then runs.

  ```python ok
  from contextlib import suppress


  def first(items: list[int]) -> int | None:
      with suppress(IndexError):
          return items[0]
      return None
  ```

- **A `yield`, `global` or `nonlocal` in the code**: each changes the function whether or not it runs. A `yield` makes
  the function a generator, as in the `return` then `yield` way of writing an empty one.

  ```python ok
  from collections.abc import Iterator


  def nothing() -> Iterator[int]:
      return
      yield
  ```

- **Code that only fails**: a `raise`, an `assert False`, or a call to `fail`, `assert_never` or `unreachable`, put
  after code that cannot fall through to say so.

  ```python ok
  def forever(queue: list[str]) -> None:
      while True:
          queue.pop()
      raise AssertionError("not reached")
  ```

- **A switched-off body containing only imports, annotations without values or `pass`**: `if False:` was the way to show
  names to type checkers alone before `TYPE_CHECKING`.

  ```python ok
  if False:
      from collections.abc import Sequence
  ```

### Fix

The fix deletes the statements that never run, and an `if False:` or `while False:` with no `else` along with its test,
when they sit on lines of their own and the block keeps a statement. It is unsafe: the code may be kept on purpose, as a
switch to turn back on.

## Ruff overlap

The [Ruff rule list](https://docs.astral.sh/ruff/rules/) has no general rule for unreachable statements after a
`return`.

## Other tools

- [pylint](https://github.com/pylint-dev/pylint) reports the code after a leaving statement as
  [unreachable](https://pylint.readthedocs.io/en/stable/user_guide/messages/warning/unreachable.html) (W0101), with no
  fix.
- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports it as WPS427.
- [vulture](https://github.com/jendrikseipp/vulture) and [basedpyright](https://github.com/DetachHead/basedpyright)
  (`reportUnreachable`) report it too.
