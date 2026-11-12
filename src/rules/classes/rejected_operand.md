Flags comparison and binary operator methods that return a boolean or raise `TypeError` for an operand type they do not
handle, instead of returning `NotImplemented`.

## Why is this bad?

For `a == b` or `a + b`, Python calls the method of `a`, and asks `b`'s reflected method only when the first returns
[`NotImplemented`](https://docs.python.org/3/library/constants.html#NotImplemented). Returning `False` or raising
`TypeError` stops that, so another type can never make `a == b` true or define `a + b`. When neither side handles the
pair, Python still answers `False` for `==` and raises `TypeError` for the other operators, so returning
`NotImplemented` loses nothing.

## Example

```python report
class Point:
    def __init__(self, x: int) -> None:
        self.x = x

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Point):
            return False
        return self.x == other.x

    def __hash__(self) -> int:
        return hash(self.x)
```

```python fixed
class Point:
    def __init__(self, x: int) -> None:
        self.x = x

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Point):
            return NotImplemented
        return self.x == other.x

    def __hash__(self) -> int:
        return hash(self.x)
```

## What it does

### Active on

- **Operator methods** of a class body: the six comparisons and the binary arithmetic and bitwise operators with their
  reflected forms. The method must start a guard on its second parameter, `if not isinstance(other, ...)`,
  `if type(other) is not ...` or `if type(other) != ...`, whose body is only `return True`, `return False` or
  `raise TypeError(...)`.

  ```python report
  class Vector:
      def __init__(self, x: int) -> None:
          self.x = x

      def __add__(self, other: "Vector") -> "Vector":
          if type(other) is not Vector:
              raise TypeError("cannot add")
          return Vector(self.x + other.x)
  ```

### Exceptions

- **Other exceptions and longer guards**, and methods that are not operators, are left alone.

  ```python ok
  class Vector:
      def __init__(self, x: int) -> None:
          self.x = x

      def __or__(self, other: "Vector") -> "Vector":
          if not isinstance(other, Vector):
              raise ValueError("cannot combine")
          return self
  ```

### Fix

The fix replaces the guard's body with `return NotImplemented` and is unsafe: the other operand's method may now answer
in place of the error or `False`. A guard body with a comment in it gets no fix.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
- [pylint](https://github.com/pylint-dev/pylint) asks for `NotImplemented` over `NotImplementedError` only, and leaves
  this case alone.
