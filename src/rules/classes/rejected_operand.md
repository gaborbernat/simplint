Flags comparison and binary operator methods that return a boolean or raise
[`TypeError`](https://docs.python.org/3/builtins/exceptions.html#TypeError) for an operand type they do not handle,
instead of returning `NotImplemented`.

## Why is this bad?

Returning [`NotImplemented`](https://docs.python.org/3/builtins/constants.html#NotImplemented) lets Python try the other
operand's method. Returning a boolean ends the comparison, and raising `TypeError` ends the operation immediately. If
both methods decline a comparison, `==` and `!=` fall back to object identity and ordering comparisons raise
`TypeError`. [Python's data model](https://docs.python.org/3/reference/datamodel.html#object.__lt__)

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

- **Operator methods** include the six comparisons and the binary arithmetic and bitwise operators and their reflected
  forms. The rule checks top-level `if` statements in the method body that test the other operand with
  `not isinstance(other, ...)`, `type(other) is not ...` or `type(other) != ...`. The `if` must have no `elif` or `else`
  and contain one statement that returns a boolean literal or raises `TypeError` without an explicit cause.

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

The unsafe fix replaces the `return` or `raise` statement with `return NotImplemented`. The other operand may then
supply a different result. The fix skips a statement that contains a comment.

## Other tools

- Ruff's [rule list](https://docs.astral.sh/ruff/rules/) has no equivalent for these operand checks.
- Pylint's [E0711](https://pylint.readthedocs.io/en/stable/user_guide/messages/error/notimplemented-raised.html) flags
  `raise NotImplemented`, which attempts to raise a value that is not an exception.
