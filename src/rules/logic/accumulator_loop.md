Flags a `for` loop whose only job is to add to a string, bytes or number that the line before it set to `""`, `b""` or
`0`. [`str.join()`](https://docs.python.org/3/library/stdtypes.html#str.join) or
[`sum()`](https://docs.python.org/3/library/functions.html#sum) builds the same value in one call.

## Why is this bad?

The loop takes three lines and a mutable name to say what `"".join(parts)` or `sum(sizes)` says in one. For strings it
can also be slow: each `+=` may copy the text built so far, so the loop is quadratic whenever CPython cannot extend the
string in place, such as for an attribute or under another interpreter.

## Example

```python report
def render(parts):
    text = ""
    for part in parts:
        text += part
    return text
```

Use instead:

```python ok
def render(parts):
    text = "".join(parts)
    return text
```

## What it does

### Active on

- **An empty `""` or `b""` start followed by a loop whose body is `name += value`**, directly or under one `if` with no
  `else`. **Unsafe fix:** both statements become `name = "".join(...)` with a generator expression, or with the iterable
  alone when the loop adds its own variable. The fix is offered when the loop has no comments, nothing reads its
  variables after it, and the value and condition neither `yield` nor `await`: a generator expression rejects the first
  and turns asynchronous with the second. It is unsafe because a `str` subclass with its own `__radd__`, such as
  `markupsafe.Markup`, gets to escape or reject each value in the loop, and `join()` skips it.

  ```python report
  def render(parts):
      text = ""
      for part in parts:
          text += part.upper()
      return text
  ```

  ```python fixed
  def render(parts):
      text = "".join(part.upper() for part in parts)
      return text
  ```

- **A `0` start**, rewritten to `sum(...)`. The fix is unsafe because a value's `__radd__` may differ from what `sum()`
  does with it.

  ```python report
  def count(parts):
      total = 0
      for part in parts:
          if part:
              total += 1
      return total
  ```

  ```python fixed
  def count(parts):
      total = sum(1 for part in parts if part)
      return total
  ```

### Exceptions

- **Loops that do anything else**: a second statement, an `else` clause, or a value that reads the accumulator.

  ```python ok
  def render(parts):
      text = ""
      for part in parts:
          text += part
          print(part)
      return text
  ```

- **A start that is not empty**, or a statement between the start and the loop.

  ```python ok
  def quote(parts):
      text = "> "
      for part in parts:
          text += part
      return text
  ```

## Other tools

- [Pylint](https://github.com/pylint-dev/pylint) reports the string form as
  [consider-using-join](https://pylint.readthedocs.io/en/stable/user_guide/messages/refactor/consider-using-join.html)
  (R1713), with no fix.
- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports the number form as
  [WPS519](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/refactoring.html#wemake_python_styleguide.violations.refactoring.ImplicitSumViolation),
  with no fix.
