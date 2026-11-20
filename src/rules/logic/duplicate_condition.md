Flags a repeated test in an `if` chain, a repeated operand in one `and` or `or` expression, or a repeated `case` pattern
when the earlier case has no guard or has the same guard.

## Why is this bad?

If a test gives the same answer twice, an `elif` that repeats it cannot run. Check whether the second test should
compare a different value, such as `"b"` below. A repeated operand in a boolean expression or a duplicate `case` pattern
can hide the same mistake. The rule compares syntax, so it can flag expressions that change value between evaluations,
such as a property with a side effect.

## Example

```python report
def label(kind):
    if kind == "a":
        return "first"
    elif kind == "a":
        return "second"
    return "other"
```

Use instead:

```python ok
def label(kind):
    if kind == "a":
        return "first"
    elif kind == "b":
        return "second"
    return "other"
```

## What it does

### Active on

- **A test repeated in an `if`/`elif` chain**, compared by structure.

  ```python report
  def label(kind, ready):
      if kind == "a":
          return 1
      elif ready:
          return 2
      elif kind == "a":
          return 3
      return 0
  ```

- **An operand repeated in one `and` or `or`.**

  ```python report
  def both_ready(first, second):
      return first.ready and second.ready and first.ready
  ```

- **A `case` pattern an earlier unguarded case has**, or an earlier case with the same guard.

  ```python report
  def describe(value):
      match value:
          case 1:
              return "one"
          case 2:
              return "two"
          case 1:
              return "uno"
      return "many"
  ```

### Exceptions

- **An `if` chain or boolean expression with a call, `await`, `yield` or assignment expression in any test or operand.**
  The rule skips the whole chain or expression because evaluation can change a later result.

  ```python ok
  def first_line(stream):
      return stream.readline() or stream.readline()
  ```

- **A test repeated immediately after its first occurrence, with the same body.** Ruff's
  [if-with-same-arms](https://docs.astral.sh/ruff/rules/if-with-same-arms/) (SIM114) can merge the branches.

  ```python ok
  def label(kind):
      if kind == "a":
          return 1
      elif kind == "a":
          return 1
      return 0
  ```

- **A repeated pattern with a different guard or no guard after a guarded case.** The first guard can fail. Identical
  guards are also exempt if they contain a call, `await`, `yield` or assignment expression.

  ```python ok
  def describe(value, strict):
      match value:
          case 1 if strict:
              return "strict one"
          case 1:
              return "one"
      return "many"
  ```

## Other tools

- Ruff's [SIM114](https://docs.astral.sh/ruff/rules/if-with-same-arms/) checks identical branch bodies.
  [PLR1714](https://docs.astral.sh/ruff/rules/repeated-equality-comparison/) recommends a membership test for
  expressions such as `x == 1 or x == 2`. [Ruff issue #21690](https://github.com/astral-sh/ruff/issues/21690) requests
  detection of repeated branch conditions.

- **[wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide):** WPS533 checks repeated
  branch conditions, WPS408 checks repeated boolean operands, and WPS535 checks repeated case patterns.
