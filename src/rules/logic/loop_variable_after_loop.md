Flags a `for` loop's variable read after the loop ends, when nothing binds it before the loop and the loop has no
`break`.

## Why is this bad?

After a loop that runs to the end, its variable holds the last item, which is rarely what the code after it means to
read. When the iterable is empty the loop never binds the variable, and the read raises `NameError`, or picks up a value
an earlier loop left behind. Naming the value the code wants, such as `items[-1]` or a variable set in the loop, says
what is meant and works for an empty iterable.

## Example

```python report
def last_row(rows: list[str]) -> str:
    for row in rows:
        print(row)
    return row
```

Use instead:

```python ok
def last_row(rows: list[str]) -> str:
    for row in rows:
        print(row)
    return rows[-1]
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

- **A name bound or read before the loop**: it keeps a value of its own when the loop runs zero times.

  ```python ok
  def last(values: list[int]) -> int | None:
      value = None
      for value in values:
          print(value)
      return value
  ```

- **A name the loop's `else` binds**: the `else` runs whenever the loop ends without a `break`.

- **A name a nested function, class or lambda mentions, one `global` or `nonlocal` declares, or a function that reads
  its locals by name**.

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
- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports a loop variable used
  after its block as WPS441.
