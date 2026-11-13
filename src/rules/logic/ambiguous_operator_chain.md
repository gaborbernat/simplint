Flags a chained comparison that mixes operator kinds, such as `key == value in table`, and one whose middle operand is a
bitwise operation written without parentheses, such as `a > 1 & b < 2`.

## Why is this bad?

Python [joins a comparison chain with `and`](https://docs.python.org/3/reference/expressions.html#comparisons):
`a op1 b op2 c` is `a op1 b and b op2 c`. That reads as intended for `a < b <= c`, but `key == value in table` means
`key == value and value in table`, and `a != b != c` never compares `a` with `c`. A bitwise operator binds tighter than
a comparison, so the NumPy or pandas mask `a > 1 & b < 2` is `a > (1 & b) < 2`, not `(a > 1) & (b < 2)`.

## Example

```python report
def has_entry(key, value, table):
    return key == value in table
```

Use instead:

```python ok
def has_entry(key, value, table):
    return key == value and value in table
```

## What it does

### Active on

- **A chain that mixes kinds or turns around**: `in` or `not in` anywhere in a chain, `is` with another operator,
  ordering with equality, or `<` with `>`.

  ```python report
  def valid(low, value):
      return 0 < low is None
  ```

- **A chain with two `!=`**: `a != b != c` does not compare `a` with `c`.

  ```python report
  def distinct(first, second, third):
      return first != second != third
  ```

- **A bitwise operation as a middle operand**, without parentheses.

  ```python report
  def mask(frame):
      return frame.a > 1 & frame.b < 2
  ```

### Exceptions

- **Chains that read as written**: one direction of ordering (`a < b <= c`), equality with at most one `!=`
  (`a == b != 0`), and `is` alone (`a is b is None`).

  ```python ok
  def in_range(low, value, high):
      return low <= value < high
  ```

- **A single comparison with a bitwise operand**, such as `flags & MASK == 0` or `mode == A | B`, which mean what they
  say.

  ```python ok
  def is_even(number):
      return number & 1 == 0
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** no rule.
- **[pylint](https://github.com/pylint-dev/pylint):**
  [bad-chained-comparison](https://pylint.readthedocs.io/en/stable/user_guide/messages/warning/bad-chained-comparison.html)
  (W3601).
- **[wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide):** WPS465, WPS409, WPS311
  and WPS532.
