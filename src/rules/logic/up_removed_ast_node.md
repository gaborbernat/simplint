Flags references to five `ast` node classes that Python 3.14 removed and visitor methods named for those classes.

## Why is this bad?

Since Python 3.8, the parser represents strings, bytes, numbers, booleans, `None` and ellipses with
[`ast.Constant`](https://docs.python.org/3/library/ast.html#ast.Constant). Python 3.14 removed the compatibility classes
`ast.Num`, `ast.Str`, `ast.Bytes`, `ast.NameConstant` and `ast.Ellipsis`. From that version,
[`ast.NodeVisitor`](https://docs.python.org/3/library/ast.html#ast.NodeVisitor) dispatches these literals to
`visit_Constant` without falling back to the older method names.

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

The rule reports on every target version. Python 3.7 still parses literals as distinct node types, so code supporting
3.7 may need the older visitor methods and type checks.

## What it does

### Active on

- **References to `ast.Num`, `ast.Str`, `ast.Bytes`, `ast.NameConstant` and `ast.Ellipsis`**, by their qualified names.
  The fixes cover two forms:

  - **Safe fix:** [`isinstance(name, ast.Str)`](https://docs.python.org/3/builtins/functions.html#isinstance) and
    `isinstance(name, ast.Bytes)` become `isinstance(name, ast.Constant) and isinstance(name.value, str)`, or `bytes`.
    Inside a larger expression the new test is wrapped in parentheses.
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
  directly subclasses `ast.NodeVisitor` or
  [`ast.NodeTransformer`](https://docs.python.org/3/library/ast.html#ast.NodeTransformer). No fix is offered: the method
  has to become `visit_Constant`, which takes over the values of all five classes.

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

  ```python ok
  from ast import Str


  def is_text(node):
      return isinstance(node, Str)
  ```

- **Classes that subclass a visitor indirectly**: only the class that names `ast.NodeVisitor` or `ast.NodeTransformer`
  as a base is checked.

  ```python ok
  import ast


  class Base(ast.NodeVisitor):
      pass


  class Counter(Base):
      def visit_Num(self, node):
          self.count += 1
  ```

### Fix

simplint marks the two rewrites above as safe. The `isinstance` rewrite assumes that `str` and `bytes` refer to the
builtins and that each `Constant` has a `value` field. A shadowed name or an incomplete node constructed by hand can
change the result.

simplint withholds the fix in these cases.

- The `isinstance` test names `Num`, `NameConstant` or `Ellipsis`. For example, the numerical check excludes `bool`.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import ast


  def is_number(node):
      return isinstance(node, ast.Num)
  ```

- The subject of the `isinstance` test is more than a name, so evaluating it twice could change the result.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import ast


  def is_text(nodes):
      return isinstance(nodes[0], ast.Str)
  ```

- A binding hides the builtin `isinstance`.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import ast


  def is_text(node, isinstance=None):
      return isinstance(node, ast.Str)
  ```

  ```python fixed
  import ast


  def is_text(node, isinstance=None):
      return isinstance(node, ast.Str)
  ```

- The constructor has positional arguments or anything other than the single expected keyword.

- The call contains a comment.
