Flags an expression nested deeper than the limit, which is 8 by default. Each operator, comparison, call, subscript,
conditional expression, lambda and comprehension adds one level to the parts below it.

## Why is this bad?

A reader evaluates a nested expression from the inside out and holds every partial result in mind until the outer parts
consume it, so each level adds to what the reader must hold while reading the next. A deep expression also hides its intermediate values from a debugger and from a stack trace, which point at the line, not at the part that
failed. A variable or a helper names each part, so the reader follows one step at a time and the debugger shows each
value.

## Example

```toml config
[tool.simplint.logic]
max-expression-complexity = 3
```

```python report
def average(prices: list[float], counts: list[int]) -> float:
    return sum(price * count for price, count in zip(prices, counts) if count > 0) / len(prices)
```

Use instead:

```python ok
def average(prices: list[float], counts: list[int]) -> float:
    total = sum(price * count for price, count in zip(prices, counts))
    return total / len(prices)
```

## Configuration

- [`logic.max-expression-complexity`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#logic)
  in `[tool.simplint]`: the deepest nesting an expression may have. An expression nested deeper is reported, such as
  `max-expression-complexity = 4` under `[tool.simplint.logic]`

  ```toml config
  [tool.simplint.logic]
  max-expression-complexity = 4
  ```

  ```python ok
  def average(prices: list[float], counts: list[int]) -> float:
      return sum(price * count for price, count in zip(prices, counts) if count > 0) / len(prices)
  ```

## What it does

The rule scores each expression a statement holds as a whole: the value of an assignment, a `return`, a call on its own
line, the test of an `if` or a `while`, and the like. The score is the deepest path from the expression down to a name
or a literal, where each of these adds one:

- a boolean, binary or unary operator, and a comparison
- a call and a subscript
- a conditional expression, a lambda and an assignment expression
- a list, set or dict comprehension and a generator expression

A name, a literal, an attribute and a collection display add nothing. A chain of the same binary operator counts once,
as `a + b + c` reads as one sum.

### Active on

- **An expression over the limit**: it is reported once, at the whole expression, so a nested part is not reported
  again.

  ```toml config
  [tool.simplint.logic]
  max-expression-complexity = 2
  ```

  ```python report
  def label(scores: dict[str, int], name: str) -> str:
      return "pass" if scores.get(name, 0) > 50 else "fail"
  ```

- **A condition**: the test of an `if` counts like any other expression.

  ```toml config
  [tool.simplint.logic]
  max-expression-complexity = 2
  ```

  ```python report
  def check(values: list[int]) -> None:
      if len(values) > 0 and max(values) > 10:
          raise ValueError(values)
  ```

### Exceptions

- **A long chain of one operator**: the left operand of the same binary operator continues the chain.

  ```toml config
  [tool.simplint.logic]
  max-expression-complexity = 1
  ```

  ```python ok
  def total(a: int, b: int, c: int, d: int) -> int:
      return a + b + c + d
  ```

- **Attributes and collection displays**: neither adds a level.

  ```toml config
  [tool.simplint.logic]
  max-expression-complexity = 0
  ```

  ```python ok
  import os

  PATHS = [os.sep, (os.curdir, {os.pardir: os.extsep})]
  ```

- **Annotations and type aliases**: a type expression reads as a type, not as a computation.

  ```toml config
  [tool.ruff]
  target-version = "py312"

  [tool.simplint.logic]
  max-expression-complexity = 0
  ```

  ```python ok
  type Matrix = list[list[float]]

  ROWS: dict[str, list[tuple[int, str]]] = {}
  ```

- **An `if` test with more boolean operands than Ruff's limit**: Ruff's too-many-boolean-expressions (PLR0916) reports
  it, as its `max-bool-expr` setting allows.

  ```toml config
  [tool.ruff.lint.pylint]
  max-bool-expr = 1

  [tool.simplint.logic]
  max-expression-complexity = 1
  ```

  ```python ok
  def check(first: int, second: int) -> None:
      if first > 0 and second > 0:
          raise ValueError(first)
  ```

### Fix

No fix is offered: naming the parts of an expression takes names only the author can choose.

## Ruff overlap

Ruff's [too-many-boolean-expressions](https://docs.astral.sh/ruff/rules/too-many-boolean-expressions/) (PLR0916) counts
the boolean operands of an `if` test. This rule leaves such a test to Ruff once Ruff's count is over its `max-bool-expr`
limit, and scores every other expression.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
- [flake8-expression-complexity](https://github.com/best-doctor/flake8-expression-complexity) reports an expression over
  its limit as ECE001, with no fix. It weighs some nodes more than one and counts attributes; this rule adds one per
  level and flattens chains of one operator.
- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports a line over a
  complexity limit as
  [WPS221](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/complexity.html#wemake_python_styleguide.violations.complexity.LineComplexityViolation),
  with no fix.
