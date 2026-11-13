Flags a comprehension with more than one `for` clause or more than one `if` clause.

## Why is this bad?

A comprehension reads as one loop and one filter. A second `for` clause nests a loop, and the
[language reference](https://docs.python.org/3/reference/expressions.html#displays-for-lists-sets-and-dictionaries)
orders the clauses outermost first, while the element comes before all of them, so the reader holds the element in mind
while reading every loop around it. [PEP 798](https://peps.python.org/pep-0798/#rationale) notes that students often
reverse the two `for` clauses, and the
[Google Python Style Guide](https://google.github.io/styleguide/pyguide.html#27-comprehensions--generator-expressions)
does not permit more than one `for` clause or filter. A second `if` clause is an `and` written as two filters, which a
named predicate states once. A `for` statement or a helper names each step.

## Example

```python report
def cells(rows: list[list[int]]) -> list[int]:
    return [cell for row in rows for cell in row]
```

Use instead:

```python ok
import itertools


def cells(rows: list[list[int]]) -> list[int]:
    return list(itertools.chain.from_iterable(rows))
```

## What it does

### Active on

List, set and dict comprehensions and generator expressions.

- **A second `for` clause**: reported at that clause.

  ```python report
  def pairs(names: list[str], sizes: list[int]) -> list[tuple[str, int]]:
      return [(name, size) for name in names for size in sizes]
  ```

- **A second `if` clause**: reported at that clause.

  ```python report
  def small_evens(values: list[int]) -> set[int]:
      return {value for value in values if value % 2 == 0 if value < 10}
  ```

### Exceptions

- **One `if` joining conditions**: `and` and `or` in one filter read as one condition.

  ```python ok
  def visible(lines: list[str]) -> list[str]:
      return [line for line in lines if line and not line.isspace()]
  ```

- **A second `for` that only unpacks, from Python 3.15**: up-comprehension-unpacking (SMP508) folds it into the element.

  ```toml config
  [tool.ruff]
  target-version = "py315"
  ```

  ```python ok
  def cells(rows: list[list[int]]) -> list[int]:
      return [cell for row in rows for cell in row]
  ```

### Fix

No fix is offered: the replacement, a `for` statement, `itertools` or a helper, depends on the comprehension.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports too many `for` clauses
  as
  [WPS224](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/complexity.html#wemake_python_styleguide.violations.complexity.TooManyForsInComprehensionViolation)
  and more than one `if` clause as
  [WPS307](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/consistency.html#wemake_python_styleguide.violations.consistency.MultipleIfsInComprehensionViolation),
  both with no fix.
