Flags a list, set or dict comprehension used as a statement only for the calls in it, and an `and` or `or` used as a
statement to decide whether to run its last part.

## Why is this bad?

A comprehension builds a collection, and one used as a statement builds it only to throw it away, so the reader looks
for the result that is never kept. `a and f()` hides a branch inside an expression: it reads as a value, while it means
"call `f()` when `a` is true". A `for` loop and an `if` say what the code does.

## Example

```python report
def notify(users: list[str], verbose: bool) -> None:
    [print(user) for user in users]
    verbose and print("sent")
```

```python fixed
def notify(users: list[str], verbose: bool) -> None:
    for user in users:
        print(user)
    if verbose:
        print("sent")
```

## What it does

### Active on

A statement that is only an expression, which is either:

- **A list, set or dict comprehension whose element calls something**, awaits or yields.

- **An `and` or `or` whose last operand calls something**, awaits or yields.

  ```python report
  def ensure(path: str, made: bool) -> None:
      made or print(path)
  ```

  ```python fixed
  def ensure(path: str, made: bool) -> None:
      if not made:
          print(path)
  ```

### Exceptions

- **A comprehension or `and`/`or` whose parts call nothing**: Ruff's
  [useless-expression](https://docs.astral.sh/ruff/rules/useless-expression/) (B018) reports a statement with no effect.

  ```python ok
  def check(values: list[int]) -> None:
      print([value for value in values])
  ```

### Fix

A comprehension becomes nested `for` loops, with an `if` for each condition, around its element. The loop variables then
stay bound after the loop, where a comprehension keeps them to itself, so the fix is unsafe. A dict comprehension is
reported without a fix, as its key and value would become two statements.

`a and f()` becomes `if a: f()`, and `a or f()` becomes `if not a: f()`, both safe. An `or` of three or more parts
becomes `if not (a or b): f()`, which is unsafe: it tests the truth of `b` a second time.

The fix applies when the statement sits on lines of its own with no comment in it.

## Ruff overlap

Ruff's [useless-expression](https://docs.astral.sh/ruff/rules/useless-expression/) (B018) reports an expression
statement without calls and skips those with calls, which this rule covers.

## Other tools

- [pylint](https://github.com/pylint-dev/pylint) reports both as
  [expression-not-assigned](https://pylint.readthedocs.io/en/stable/user_guide/messages/warning/expression-not-assigned.html)
  (W0106), with no fix.
