Flags methods that override a method of a base class in the run without
[`@typing.override`](https://docs.python.org/3/library/typing.html#typing.override).

## Why is this bad?

An override depends on the base keeping the method. When the base renames or removes it, the subclass's method stops
overriding anything and the base's callers stop reaching it, with no error anywhere. `@override`, which
[PEP 698](https://peps.python.org/pep-0698/) added in Python 3.12, states the intent, so a type checker reports the
method once the base no longer has it. It also tells a reader the method is part of the base's contract rather than the
subclass's own.

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

The rule follows each module-level class's bases through the classes of the run, across files and imports, and reports a
method whose name a class up the chain defines as a method.

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

- **Properties**: the getter gets the decorator; its setter shares the name and is left alone.

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

- **Special methods** such as `__init__` and `__repr__`: every class inherits them from `object`, so defining one is how
  a class joins a protocol rather than a choice to override a base.

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

The fix adds `@override` right above the `def`, below the other decorators, so `@property`, `@classmethod` and
`@staticmethod` wrap the marked function, and imports `override` from `typing`. The decorator returns the function
unchanged, so the fix is safe. When another binding holds the name `override`, the rule reports without a fix.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** no rule.
- [pyright](https://github.com/microsoft/pyright) and [basedpyright](https://github.com/DetachHead/basedpyright) report
  the same methods as `reportImplicitOverride`, off by default, and include special methods and bases outside the
  project, which they resolve through type information.
- [mypy](https://github.com/python/mypy) reports them with the `explicit-override` error code, off by default.
