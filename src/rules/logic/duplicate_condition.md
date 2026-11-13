Flags a test that an earlier branch of the same `if` chain already tried, an operand repeated in one `and` or `or`, and
a `case` pattern an earlier case already matches.

## Why is this bad?

A repeated test is a copy-paste slip. An `elif` whose test an earlier branch already tried only runs when that test was
false, so the branch never runs, and the check that was meant, such as `kind == "b"`, is missing. A repeated operand of
`and` or `or` never changes the result, and a `case` after an identical unguarded one never matches.

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

- **Tests and operands with a call, `await`, `yield` or walrus**: `read() or read()` reads twice and can differ.

  ```python ok
  def first_line(stream):
      return stream.readline() or stream.readline()
  ```

- **Two adjacent branches with the same body**: Ruff's
  [if-with-same-arms](https://docs.astral.sh/ruff/rules/if-with-same-arms/) (SIM114) reports them and merges them.

  ```python ok
  def label(kind):
      if kind == "a":
          return 1
      elif kind == "a":
          return 1
      return 0
  ```

- **A `case` after a guarded one with the same pattern**: the guard can fail, so the later case can match.

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

- **[Ruff](https://github.com/astral-sh/ruff):**
  [if-with-same-arms](https://docs.astral.sh/ruff/rules/if-with-same-arms/) (SIM114) merges adjacent branches with the
  same body, and [repeated-equality-comparison](https://docs.astral.sh/ruff/rules/repeated-equality-comparison/)
  (PLR1714) turns `x == 1 or x == 2` into a membership test; neither reports a repeated test.
  [Ruff issue #21690](https://github.com/astral-sh/ruff/issues/21690) asks for this rule.
- **[wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide):** WPS533, WPS408 and
  WPS535.
