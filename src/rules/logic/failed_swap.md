Flags a swap that assigns one value to both names, `a = b` followed by `b = a`, and an augmented assignment that repeats
its target, such as `total += total + delta`.

## Why is this bad?

After `a = b`, `b = a` stores `b` back into itself, so both names hold the old `b` and the old `a` is lost. Python swaps
in one statement, `a, b = b, a`, which evaluates the right side first. `total += total + delta` adds `total` twice,
which can double a running total by mistake. A swap test with two equal values misses the lost value.

## Example

```python report
def flip(node):
    node.left = node.right
    node.right = node.left
```

Use instead:

```python ok
def flip(node):
    node.left, node.right = node.right, node.left
```

## What it does

### Active on

- **`a = b` directly followed by `b = a`**, where both sides are names or attributes. **Unsafe fix:** the two statements
  become one tuple assignment, which swaps the values. The replacement changes the result. A comment between the two
  statements prevents the fix.

  ```python report
  def flip(first, second):
      first = second
      second = first
      return first, second
  ```

  ```python fixed
  def flip(first, second):
      first, second = second, first
      return first, second
  ```

- **`x op= x op ...` with the same operator**, where `x` is a name or attribute.

  ```python report
  def grow(total, delta):
      total += total + delta
      return total
  ```

### Exceptions

- **A different operator on the right**: `total += total * rate` can mean what it says.

  ```python ok
  def grow(total, rate):
      total += total * rate
      return total
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):**
  [swap-with-temporary-variable](https://docs.astral.sh/ruff/rules/swap-with-temporary-variable/) (PLR1712) rewrites a
  swap through a temporary; it does not report the two-statement form.
- wemake-python-styleguide reports failed swaps as
  [WPS523](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/refactoring.html#wemake_python_styleguide.violations.refactoring.AlmostSwappedViolation)
  and repeated operands in augmented assignments as
  [WPS524](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/refactoring.html#wemake_python_styleguide.violations.refactoring.MisrefactoredAssignmentViolation).
