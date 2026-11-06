Flags references to the `ast` node aliases that Python 3.14 removed, and to `visit_` methods that no longer run.

## Why is this bad?

`ast.Num`, `ast.Str`, `ast.Bytes`, `ast.NameConstant` and `ast.Ellipsis` have been aliases of `ast.Constant` since
Python 3.8, and Python 3.14 removes them, so code that names them fails there with `AttributeError`. A visitor method
named after one of them is no longer called, because `ast.NodeVisitor` hands a constant only to `visit_Constant`.

## Example

```toml config
[project]
requires-python = ">=3.10"
```

```python report
import ast


def is_text(node):
    return isinstance(node, ast.Str)


def make_number():
    return ast.Num(n=1)
```

Use instead:

```python ok
import ast


def is_text(node):
    return isinstance(node, ast.Constant) and isinstance(node.value, str)


def make_number():
    return ast.Constant(value=1)
```

## Configuration

No setting changes the report. The aliases are gone from Python 3.14, and their deprecation covers every version
simplint supports, so the rule reports on every target version.

## What it does

### Active on

- **References to `ast.Num`, `ast.Str`, `ast.Bytes`, `ast.NameConstant` and `ast.Ellipsis`**, by their qualified names.
  The fixes cover two forms:

  - **Safe fix:** `isinstance(name, ast.Str)` and `isinstance(name, ast.Bytes)` become
    `isinstance(name, ast.Constant) and isinstance(name.value, str)`, or `bytes`. Inside a larger expression the new
    test is wrapped in parentheses.
  - **Safe fix:** `ast.Num(n=...)`, `ast.Str(s=...)`, `ast.Bytes(s=...)` and `ast.NameConstant(value=...)` with that one
    keyword become `ast.Constant(value=...)`.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import ast


  def is_text(node):
      return isinstance(node, ast.Str)
  ```

  ```python fixed
  import ast


  def is_text(node):
      return isinstance(node, ast.Constant) and isinstance(node.value, str)
  ```

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import ast


  def make_number():
      return ast.Num(n=1)
  ```

  ```python fixed
  import ast


  def make_number():
      return ast.Constant(value=1)
  ```

- **`visit_Num`, `visit_Str`, `visit_Bytes`, `visit_NameConstant` and `visit_Ellipsis` methods** of a class that
  directly subclasses `ast.NodeVisitor` or `ast.NodeTransformer`. No fix is offered: the method has to become
  `visit_Constant`, which takes over the values of every one of those classes.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import ast


  class Counter(ast.NodeVisitor):
      def visit_Num(self, node):
          self.count += 1
  ```

### Exceptions

- **Names imported from `ast`**, such as `from ast import Str`: the fix would need a new import, so these are not
  reported.
- **Classes that subclass a visitor indirectly**: only the class that names `ast.NodeVisitor` or `ast.NodeTransformer`
  as a base is checked.

### Fix

No fix is offered when:

- the `isinstance` test names `Num`, `NameConstant` or `Ellipsis`, whose checks go beyond the value's type: `Num` leaves
  out `bool`;

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import ast


  def is_number(node):
      return isinstance(node, ast.Num)
  ```

- the subject of the `isinstance` test is not a name, because the rewrite repeats it;

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import ast


  def is_text(nodes):
      return isinstance(nodes[0], ast.Str)
  ```

- the constructor takes positional arguments or more than one keyword, or the keyword is not the value's name;

- the call has a comment, which the rewrite would drop.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
