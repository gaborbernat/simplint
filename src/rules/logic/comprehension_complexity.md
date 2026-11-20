Flags a comprehension with more than two `for` clauses or more than one `if` attached to the first `for`.

## Why is this bad?

In a [comprehension](https://docs.python.org/3/reference/expressions.html#displays-for-lists-sets-and-dictionaries), the
result expression appears before the clauses that bind its names. With three loops, explicit `for` statements can make
it easier to see where each value comes from. Multiple filters on the same loop can often be combined with `and` or
named in a helper.

## Example

```python report
def cells(grid: list[list[list[int]]]) -> list[int]:
    return [cell for plane in grid for row in plane for cell in row]
```

Use instead:

```python ok
def cells(grid: list[list[list[int]]]) -> list[int]:
    found = []
    for plane in grid:
        for row in plane:
            found.extend(row)
    return found
```

## What it does

### Active on

List, set and dict comprehensions and generator expressions.

- **A third `for` clause**: reported at that clause.

  ```python report
  def triples(names: list[str], sizes: list[int], flags: list[bool]) -> list[tuple[str, int, bool]]:
      return [(name, size, flag) for name in names for size in sizes for flag in flags]
  ```

- **A second filter on the first loop.** The diagnostic points to its condition. Filters attached to a second loop do
  not count toward this limit.

  ```python report
  def small_evens(values: list[int]) -> set[int]:
      return {value for value in values if value % 2 == 0 if value < 10}
  ```

### Exceptions

- **Two `for` clauses**: a pairing of two sequences reads as one step.

  ```python ok
  def pairs(names: list[str], sizes: list[int]) -> list[tuple[str, int]]:
      return [(name, size) for name in names for size in sizes]
  ```

- **One `if` joining conditions**: `and` and `or` in one filter read as one condition.

  ```python ok
  def visible(lines: list[str]) -> list[str]:
      return [line for line in lines if line and not line.isspace()]
  ```

### Fix

No fix is offered: the replacement, a `for` statement, `itertools` or a helper, depends on the comprehension.

## Other tools

- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports more than two `for`
  clauses as
  [WPS224](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/complexity.html#wemake_python_styleguide.violations.complexity.TooManyForsInComprehensionViolation)
  and more than one `if` clause as
  [WPS307](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/consistency.html#wemake_python_styleguide.violations.consistency.MultipleIfsInComprehensionViolation),
  both with no fix.
