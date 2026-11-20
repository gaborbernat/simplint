Flags a chained comparison that mixes operator kinds, such as `key == value in table`, and one whose middle operand is a
bitwise operation written without parentheses, such as `a > 1 & b < 2`.

## Why is this bad?

Python [compares adjacent operands](https://docs.python.org/3/reference/expressions.html#comparisons) in a chain and
evaluates each at most once. So `key == value in table` tests both `key == value` and `value in table`, and
`a != b != c` never compares `a` with `c`. Bitwise operators bind more tightly than comparisons. To combine two array
comparisons into a mask, write `(a > 1) & (b < 2)`. Without the parentheses, Python compares both ends with `1 & b`.

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

- **A chain that mixes kinds or turns around**: `in`, `not in` or `is not` anywhere in a chain, `is` with another
  operator, ordering with equality, or `<` with `>`.

  ```python report
  def valid(low, value):
      return 0 < low is None
  ```

- **A chain with two `!=`**: `a != b != c` does not compare `a` with `c`.

  ```python report
  def distinct(first, second, third):
      return first != second != third
  ```

- **An unparenthesized `&`, `|` or `^` expression as a middle operand.**

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

- **A single comparison with a bitwise operand**, such as `flags & MASK == 0` or `mode == A | B`.

  ```python ok
  def is_even(number):
      return number & 1 == 0
  ```

## Other tools

- **[pylint](https://github.com/pylint-dev/pylint):**
  [bad-chained-comparison](https://pylint.readthedocs.io/en/stable/user_guide/messages/warning/bad-chained-comparison.html)
  (W3601).
- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) checks heterogeneous
  comparisons (WPS409), repeated membership tests (WPS311) and chained identity comparisons (WPS532).
