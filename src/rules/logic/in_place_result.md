Flags the result of a method that changes a list, dict or set in place, such as `ordered = names.sort()`, and of
`random.shuffle()`, which are always `None`.

## Why is this bad?

[`list.sort()`](https://docs.python.org/3/library/stdtypes.html#list.sort), `list.append()`, `dict.update()`,
`set.add()` and the other in-place methods change their object and return `None`, as does
[`random.shuffle()`](https://docs.python.org/3/library/random.html#random.shuffle). Storing, returning, passing or
comparing that result works with `None`, which
[`sorted()`](https://docs.python.org/3/library/functions.html#sorted) was likely meant to avoid. A comparison of two
such results is always true.

## Example

```python report
def same_keys(first, second):
    return list(first).sort() == list(second).sort()
```

Use instead:

```python ok
def same_keys(first, second):
    return sorted(first) == sorted(second)
```

## What it does

### Active on

- **A stored, returned, passed or compared result** of `append`, `extend`, `insert`, `remove`, `reverse`, `clear` or
  `sort` on a list, `update` or `clear` on a dict, and `add`, `update`, `discard`, `remove`, `clear` and the
  `*_update` methods on a set. The object must be known to be one: a display, a comprehension, a `list()`, `dict()` or
  `set()` call, a name bound only to one of those, or a parameter annotated with the type.

  ```python report
  def collect(items: list[int]):
      added = items.append(1)
      return added
  ```

- **`random.shuffle()`**, whatever it shuffles.

  ```python report
  import random


  def deal(cards):
      hand = random.shuffle(cards)
      return hand[:5]
  ```

### Exceptions

- **An object of unknown type**: `pyarrow.Array.sort()` and Django's `QuerySet.update()` return values.

  ```python ok
  def ordered(array):
      return array.sort()
  ```

- **A lambda, and a function whose whole body returns the call**: callbacks such as `lambda value: seen.append(value)`
  throw the value away on purpose.

  ```python ok
  def recorder(calls: list[dict]):
      def record(**kwargs):
          return calls.append(kwargs)

      return record
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** no rule.
- **[pylint](https://github.com/pylint-dev/pylint):**
  [assignment-from-none](https://pylint.readthedocs.io/en/stable/user_guide/messages/error/assignment-from-none.html)
  (E1128) through type inference.
