Flags list or tuple repetition such as `grid = [[0] * n] * m` when the code also changes an element through the same
name or attribute.

## Why is this bad?

[Sequence repetition](https://docs.python.org/3/builtins/stdtypes.html#common-sequence-operations) copies references to
the original elements. After `grid = [[0] * 3] * 3`, all three rows refer to one list, so `grid[0][0] = 1` changes the
first cell in every row. A comprehension can build each row separately, as the
[Python FAQ](https://docs.python.org/3/faq/programming.html#how-do-i-create-a-multidimensional-list) illustrates.

## Example

```python report
def board(size):
    grid = [[0] * size] * size
    grid[0][0] = 1
    return grid
```

Use instead:

```python ok
def board(size):
    grid = [[0] * size for _ in range(size)]
    grid[0][0] = 1
    return grid
```

## What it does

### Active on

- **A repetition stored in a name or attribute with an element mutation in the same scope**: the element is a list, dict
  or set display, a comprehension, a call such as `list()` or `dict()`, or an expression that adds or repeats a list
  literal. A change is an assignment or `del` of a part of an element (`grid[i][j] = 1`, `cells[i].name = 1`), an
  augmented assignment to an element (`rows[i] += [x]`), or a call of a method that changes it in place
  (`rows[i].append(x)`). A local name is checked throughout its function. For a module name or an attribute such as
  `self.cells`, simplint checks the whole file. The mutation can appear before the assignment. **Unsafe fix:** a list of
  one element becomes a comprehension over `range()`, which builds the element once per copy. A call in the element then
  runs once per copy, which can change what it builds.

  ```python report
  class Table:
      def __init__(self, size):
          self.cells = [{}] * size

      def put(self, index, key):
          self.cells[index][key] = 1
  ```

  ```python fixed
  class Table:
      def __init__(self, size):
          self.cells = [{} for _ in range(size)]

      def put(self, index, key):
          self.cells[index][key] = 1
  ```

### Exceptions

- **A repetition nothing changes through its name**: data handed to a constructor such as `numpy.array()` is only read,
  so sharing the rows does not show.

  ```python ok
  def zeros(size):
      return [[0] * size] * size
  ```

- **A literal repeat count of zero or one**, since no two copies can share an element.

- **An immutable element**, such as a number, a string or `None`.

  ```python ok
  def slots(size):
      values = [None] * size
      values[0] = 1
      return values
  ```

- **No fix** for a tuple, a list of several elements, a binding that shadows `range`, or an element that reads the
  comprehension variable `_`.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** no rule.
- **[wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide):**
  [WPS435](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/best_practices.html#wemake_python_styleguide.violations.best_practices.ListMultiplyViolation)
  also reports multiplication of list literals with immutable elements.
