Flags a function whose cognitive complexity is over the limit, which is 20 by default. The score follows SonarSource's
definition: it counts the branches, loops and handlers a reader must follow, and counts each one more the deeper it is
nested.

## Why is this bad?

A function's length says little about how hard it is to read. Several branches nested in a loop are harder to follow
than a longer straight-line function, because the reader holds every condition while reading the code under it. The
score rises with that nesting, so moving a deep branch into a named helper lowers the score of the function it leaves.

## Example

```toml config
[tool.simplint.logic]
max-cognitive-complexity = 3
```

```python report
def sign(values: list[int]) -> list[str]:
    result = []
    for value in values:
        if value > 0:
            result.append("+")
        elif value < 0:
            result.append("-")
    return result
```

Use instead:

```python ok
def sign(value: int) -> str:
    if value > 0:
        return "+"
    if value < 0:
        return "-"
    return "0"


def signs(values: list[int]) -> list[str]:
    return [sign(value) for value in values]
```

## Configuration

- [`logic.max-cognitive-complexity`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#logic)
  in `[tool.simplint]`: the highest score a function may have. A function scoring more is reported, such as
  `max-cognitive-complexity = 30` under `[tool.simplint.logic]`

  ```toml config
  [tool.simplint.logic]
  max-cognitive-complexity = 30
  ```

  ```python ok
  def sign(values: list[int]) -> list[str]:
      result = []
      for value in values:
          if value > 0:
              result.append("+")
          elif value < 0:
              result.append("-")
      return result
  ```

## What it does

The score of a function adds these, where the nesting is the number of bodies of branches, loops, handlers, `case`
clauses, conditional expressions and lambdas the structure sits in:

- one for each `if`, `for`, `while`, `match`, `except` and conditional expression, plus its nesting
- one for each `elif` and `else`, the `else` of a loop or a `try` included, without the nesting
- one for each run of one boolean operator: `a and b or c` adds two, `a and b and c` adds one

As in SonarSource's Python analyzer, a condition counts at the nesting of its statement, `try`, `finally` and `with`
bodies do not nest, and comprehensions add nothing. Two choices differ from SonarSource: a nested `def` or `class` is
scored on its own, so its statements do not count for the function around it, and recursion is not counted.

### Active on

Every function of the module, nested functions included, is reported at its name when its score is over the limit.

- **A function with several nested branches**: the score counts the nested ones more. The limit is lowered here so that
  the function is over it.

  ```toml config
  [tool.simplint.logic]
  max-cognitive-complexity = 5
  ```

  ```python report
  def scale(rows: list[list[int]], limit: int) -> int:
      total = 0
      for row in rows:
          for value in row:
              if value > limit:
                  total += 1
      return total
  ```

### Exceptions

- **A function at or under the limit**: the score must be more than the limit.

  ```python ok
  def sign(value: int) -> str:
      if value > 0:
          return "+"
      return "-"
  ```

- **A comprehension**: its `for` and `if` clauses add nothing, so even a limit of zero keeps it.

  ```toml config
  [tool.simplint.logic]
  max-cognitive-complexity = 0
  ```

  ```python ok
  def positives(values: list[int]) -> list[int]:
      return [value for value in values if value > 0]
  ```

- **A nested function's own statements**: they are scored with the nested function.

  ```python ok
  def outer(values: list[int]) -> int:
      def inner(value: int) -> int:
          if value:
              return 1
          return 0

      return inner(values[0])
  ```

### Fix

No fix is offered: splitting a function is a decision about its design.

## Ruff overlap

Ruff's [complex-structure](https://docs.astral.sh/ruff/rules/complex-structure/) (C901) counts McCabe's cyclomatic
complexity, which adds one per branch and does not grow with nesting. The two scores disagree, and this rule reads the
nesting that McCabe's score ignores.

## Other tools

- [flake8-cognitive-complexity](https://github.com/Melevir/flake8-cognitive-complexity) reports functions over a
  cognitive complexity limit as [CCR001](https://github.com/Melevir/flake8-cognitive-complexity), with no fix.
- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports the same score as
  [WPS231](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/complexity.html#wemake_python_styleguide.violations.complexity.CognitiveComplexityViolation),
  with no fix.
