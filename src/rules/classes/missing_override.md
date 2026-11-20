Flags methods that override a method of a base class in the run without
[`@typing.override`](https://docs.python.org/3/library/typing.html#typing.override).

## Why is this bad?

If a base class renames a method, its subclass can keep the old name without raising an error at class definition. A
type checker can flag that stale override when the author marks it with `@override`. Python 3.12 added the decorator to
`typing`. [PEP 698](https://peps.python.org/pep-0698/)

## Example

```toml config
[project]
requires-python = ">=3.12"
```

```python report
class Shape:
    def area(self) -> float:
        return 0.0


class Square(Shape):
    def area(self) -> float:
        return 4.0
```

Use instead:

```toml config
[project]
requires-python = ">=3.12"
```

```python ok
from typing import override


class Shape:
    def area(self) -> float:
        return 0.0


class Square(Shape):
    @override
    def area(self) -> float:
        return 4.0
```

## What it does

For each module-level class, simplint follows base classes defined in the checked files. A method overrides an
ancestor's method when both have the same name.

### Active on

- **Methods of a base in another file**:

  ```toml config
  [project]
  requires-python = ">=3.12"
  ```

  ```python context file=app/shapes.py
  class Shape:
      def area(self) -> float:
          return 0.0
  ```

  ```python report file=app/square.py
  from app.shapes import Shape


  class Square(Shape):
      def area(self) -> float:
          return 4.0
  ```

- **Properties** get one report on the getter. The setter and deleter share its name.

  ```toml config
  [project]
  requires-python = ">=3.12"
  ```

  ```python report
  class Shape:
      @property
      def area(self) -> float:
          return 0.0


  class Square(Shape):
      @property
      def area(self) -> float:
          return 4.0
  ```

### Exceptions

- **Special methods** such as `__init__`, `__repr__` and `__iter__` implement Python protocols. The rule skips
  double-underscore method names.

  ```toml config
  [project]
  requires-python = ">=3.12"
  ```

  ```python ok
  class Shape:
      def __repr__(self) -> str:
          return "Shape"


  class Square(Shape):
      def __repr__(self) -> str:
          return "Square"
  ```

- **Bases outside the run**, such as a standard library or third-party class: the rule does not know their methods.

  ```toml config
  [project]
  requires-python = ">=3.12"
  ```

  ```python ok
  import json


  class Encoder(json.JSONEncoder):
      def default(self, o: object) -> object:
          return str(o)
  ```

- **Overload variants**: `@override` goes on the implementation, which the rule reports when it lacks it.

  ```toml config
  [project]
  requires-python = ">=3.12"
  ```

  ```python report
  from typing import overload


  class Store:
      def get(self, key: object) -> object:
          return key


  class Strings(Store):
      @overload
      def get(self, key: int) -> int: ...

      @overload
      def get(self, key: str) -> str: ...

      def get(self, key: object) -> object:
          return key
  ```

- **Targets before Python 3.12**, where `typing` has no `override`.

### Fix

The safe fix imports `override` from `typing` if needed and inserts `@override` immediately above the `def`. That
placement gives `@property`, `@classmethod` and `@staticmethod` the marked function to wrap. A conflicting binding for
`override` prevents the fix.

## Other tools

- Ruff's [rule list](https://docs.astral.sh/ruff/rules/) has no equivalent check for unmarked overrides.
- Pyright disables `reportImplicitOverride` by default.
  [Pyright configuration](https://github.com/microsoft/pyright/blob/main/docs/configuration.md#type-check-diagnostics-settings)
- Basedpyright reports `reportImplicitOverride` as a warning in its default `recommended` mode.
  [Basedpyright configuration](https://docs.basedpyright.com/latest/configuration/config-files/#diagnostic-settings-defaults)
- Mypy provides the optional
  [`explicit-override` error code](https://mypy.readthedocs.io/en/stable/error_code_list2.html#code-explicit-override).
