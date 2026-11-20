Flags a `for` loop that starts immediately after `name = ""`, `name = b""` or `name = 0` and only adds values to that
name.

## Why is this bad?

[`str.join()`](https://docs.python.org/3/builtins/stdtypes.html#str.join),
[`bytes.join()`](https://docs.python.org/3/builtins/stdtypes.html#bytes.join) and
[`sum()`](https://docs.python.org/3/builtins/functions.html#sum) express the operation in one call. Repeated string
concatenation can take quadratic time when each addition copies the text built so far.

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

- **Strings and bytes.** The loop body can add a value directly or under a single `if` with no `else`. The unsafe fix
  uses `"".join(...)` or `b"".join(...)`. It passes the iterable directly when the loop adds each item unchanged and
  uses a generator expression for a filter or transformation. A string subclass such as
  [`markupsafe.Markup`](https://markupsafe.palletsprojects.com/en/stable/escaping/) can customize addition to escape
  text. Joining with `""` bypasses that behavior.

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

- **Numbers.** The unsafe fix replaces the loop with `sum(...)`. Floating-point sums can round differently, and
  user-defined types can implement in-place addition differently from addition.

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

The rule offers these fixes only when the loop variables are bound once and read only inside the loop. A comment in the
replaced span or a `yield` or `await` in the value or filter also prevents the fix.

### Exceptions

- **Async loops, class bodies and annotated initial assignments.** The rule checks ordinary `for` loops following a
  plain assignment at module scope or inside a function.

- **An accumulator mentioned elsewhere in the loop.** The iterable, target, filter and added value must all be
  independent of the accumulator. A `global` or `nonlocal` declaration for the name also excludes it.

- **Loops with a second statement or an `else` clause.**

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
