Flags functions with a cognitive complexity score above the configured limit (20 by default). Nested branches, loops and
exception handlers add more to the score than top-level ones.

## Why is this bad?

A reader following an `if` inside two loops has to keep track of both loops and the condition.
[SonarSource's cognitive complexity metric](https://www.sonarsource.com/docs/CognitiveComplexity.pdf) gives such a
branch more weight. A [meta-analysis of 427 snippets](https://arxiv.org/abs/2007.12520) found that the score correlates
with comprehension time and subjective ratings. [Lavazza et al.](https://irinsubria.uninsubria.it/handle/11383/2144151)
found that adding it to traditional metrics did little to improve predictions of understandability.

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
def sign(values: list[int]) -> list[str]:
    return [nonzero_sign(value) for value in values if value]


def nonzero_sign(value: int) -> str:
    return "+" if value > 0 else "-"
```

## Configuration

- [`logic.max-cognitive-complexity`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#logic)
  sets the highest allowed score. The example raises it to 30.

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

Nesting depth starts at zero. Branch bodies, loop bodies, handlers, `case` clauses, conditional expressions and lambdas
increase it.

- One plus the current depth for each `if`, `for`, `while`, `match`, `except` and conditional expression.
- One for each `elif` and `else`, including the `else` of a loop or a `try`.
- One for each run of the same boolean operator. For example, `a and b or c` adds two and `a and b and c` adds one.

An `if` or `while` condition uses the nesting depth of its statement. The bodies of `try`, `finally` and `with` add no
nesting. Comprehension clauses add no points of their own, but expressions inside them can count. Nested definitions do
not contribute to the enclosing function's score. Each function gets its own score, and recursive calls add no points.

### Active on

simplint reports a function at its name when its score exceeds the limit.

- **Two loops and an `if`.** The outer loop adds one, the inner loop adds two and the `if` adds three. The total exceeds
  the configured limit of five.

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

- **A comprehension with one comparison.** Its loop and filter add no points.

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

Ruff's [complex-structure (C901)](https://docs.astral.sh/ruff/rules/complex-structure/) measures McCabe complexity. A
branch contributes equally at any depth. [Ruff issue #2418](https://github.com/astral-sh/ruff/issues/2418) tracks the
request for cognitive complexity.

## Other tools

- [flake8-cognitive-complexity](https://github.com/Melevir/flake8-cognitive-complexity) reports functions over a
  cognitive complexity limit as [CCR001](https://github.com/Melevir/flake8-cognitive-complexity), with no fix.
- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports functions over its
  cognitive complexity limit as
  [WPS231](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/complexity.html#wemake_python_styleguide.violations.complexity.CognitiveComplexityViolation),
  with no fix.
