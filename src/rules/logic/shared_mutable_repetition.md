Flags a list or tuple repeated with `*` whose element is a mutable object, such as `[[0] * n] * m`, when the code then
changes an element through the name it is stored in.

## Why is this bad?

Repeating a sequence copies references, not objects:
[`s * n`](https://docs.python.org/3/library/stdtypes.html#common-sequence-operations) holds the same element `n` times.
The [Programming FAQ](https://docs.python.org/3/faq/programming.html#how-do-i-create-a-multidimensional-list) shows the
result: after `grid = [[0] * 3] * 3`, `grid[0][0] = 1` sets the first cell of every row, since all three rows are one
list. A comprehension builds a new element for each copy.

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

- **A repetition stored in a name or attribute whose element is then changed**: the element is a list, dict or set
  display, a comprehension, a call such as `list()` or `dict()`, or a list repeated by `*` itself. A change is an
  assignment or `del` of a part of an element (`grid[i][j] = 1`, `cells[i].name = 1`), an augmented assignment to an
  element (`rows[i] += [x]`), or a call of a method that changes it in place (`rows[i].append(x)`). A local name is
  searched in its own function; a module name or an attribute, such as `self.cells`, in the whole file. **Unsafe fix:**
  a list of one element becomes a comprehension over `range()`, which builds the element once per copy. A call in the
  element then runs once per copy, which can change what it builds.

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

- **An immutable element**, such as a number, a string or `None`.

  ```python ok
  def slots(size):
      values = [None] * size
      values[0] = 1
      return values
  ```

- **No fix** for a tuple, a list of several elements, or an element that reads `_`, which the comprehension's own
  variable would shadow.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** no rule.
- **[wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide):** WPS435 reports every list
  multiplication.
