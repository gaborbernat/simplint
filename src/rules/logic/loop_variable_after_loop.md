Flags a `for` loop's variable read after the loop ends, when nothing binds it before the loop and the loop has no
`break`.

## Why is this bad?

A loop over an empty iterable never assigns its target. Reading that local afterwards raises
[`UnboundLocalError`](https://docs.python.org/3/builtins/exceptions.html#UnboundLocalError) if it has no earlier value.
If you want the last item of a sequence, access it explicitly and decide what to return when the sequence is empty.

## Example

```python report
def last_row(rows: list[str]) -> str:
    for row in rows:
        print(row)
    return row
```

Use instead:

```python ok
def last_row(rows: list[str]) -> str | None:
    for row in rows:
        print(row)
    return rows[-1] if rows else None
```

## What it does

### Active on

A read of a name the `for` loop's target binds, in a statement after the loop in the same block, inside a function. Each
name of an unpacking target counts on its own, and the first mention of the name after the loop decides: a read is
reported, a new assignment ends the check.

### Exceptions

- **A loop with a `break`**: the variable holds the item the loop stopped at, the usual way to search.

  ```python ok
  def first_negative(values: list[int]) -> int | None:
      for value in values:
          if value < 0:
              break
      else:
          return None
      return value
  ```

- **A name bound or read before the loop.** The rule skips earlier mentions because the variable may already have a
  value.

  ```python ok
  def last(values: list[int]) -> int | None:
      value = None
      for value in values:
          print(value)
      return value
  ```

- **A loop whose `else` mentions the name or exits the block.** The rule skips these loops. An `else` can set a fallback
  value or return before the later read.

- **The placeholder `_`, names declared `global` or `nonlocal`, names captured by nested scopes, and functions that read
  their locals by name.**

- **A comprehension that binds the same name**: it has a variable of its own.

  ```python ok
  def doubled(values: list[int]) -> list[int]:
      for value in values:
          print(value)
      return [value * 2 for value in values]
  ```

## Ruff overlap

Ruff's [redefined-loop-name](https://docs.astral.sh/ruff/rules/redefined-loop-name/) (PLW2901) reports a loop variable
assigned inside the loop, and
[unused-loop-control-variable](https://docs.astral.sh/ruff/rules/unused-loop-control-variable/) (B007) one the body
never reads. Neither looks at reads after the loop.

## Other tools

- [pylint](https://github.com/pylint-dev/pylint) reports a loop variable that may be unbound after the loop as
  [undefined-loop-variable](https://pylint.readthedocs.io/en/stable/user_guide/messages/warning/undefined-loop-variable.html)
  (W0631).
- wemake-python-styleguide's
  [WPS441](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/best_practices.html#wemake_python_styleguide.violations.best_practices.ControlVarUsedAfterBlockViolation)
  covers variables from `for` and `with` blocks. It allows later reads in `assert` statements.
