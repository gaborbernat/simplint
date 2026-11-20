Flags a list, set or dict comprehension used as a statement only for the calls in it, and an `and` or `or` used as a
statement to decide whether to run its last part.

## Why is this bad?

A standalone comprehension allocates a collection and immediately discards it. A loop shows that the calls are the
purpose of the statement. Likewise, `if a: f()` makes the condition for calling `f()` explicit.

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

The rule checks these standalone expressions.

- **A list, set or dict comprehension with a call or an `await` in the element, dict key or dict value.**

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

- **An expression whose result is used**, such as a comprehension passed to another function.

  ```python ok
  def check(values: list[int]) -> None:
      print([value for value in values])
  ```

### Fix

A comprehension becomes nested `for` loops, with an `if` for each condition, around its element. The loop variables then
stay bound after the loop, where a comprehension keeps them to itself, so the fix is unsafe. A dict comprehension is
reported without a fix, as its key and value would become two statements.

`a and f()` becomes `if a: f()`, and `a or f()` becomes `if not a: f()`, both marked safe. For three or more operands,
the fix combines the earlier operands into a single negated condition, as in `if not (a or b): f()`. Simplint marks that
form unsafe.

The fix applies when the statement sits on lines of its own with no comment in it.

## Ruff overlap

Ruff's [useless-expression](https://docs.astral.sh/ruff/rules/useless-expression/) (B018) checks standalone expressions
that appear to have no effect. Simplint checks comprehensions used for their side effects and boolean expressions that
conditionally perform an action.

## Other tools

- [Pylint](https://github.com/pylint-dev/pylint) reports standalone comprehensions and boolean expressions containing a
  function call as
  [expression-not-assigned](https://pylint.readthedocs.io/en/stable/user_guide/messages/warning/expression-not-assigned.html)
  (W0106), with no fix.
