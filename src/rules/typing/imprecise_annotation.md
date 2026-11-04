Flags `object` and `Any` in annotations. Both accept any value, so the code behind them has to narrow the value at
runtime, and a type checker stops checking every value they touch.

## Why is this bad?

`object` accepts anything and permits almost no operations, so the code behind it narrows with `isinstance` or casts at
runtime. `Any` skips type checking altogether for each value it touches. A precise type moves that check to the type
checker and documents what callers pass.

## Example

```python report
def describe(value: object) -> str:
    return str(value)


CACHE: dict[str, Any] = {}
```

Use instead:

```python ok
def describe(value: int | str) -> str:
    return str(value)


CACHE: dict[str, int] = {}
```

## What it does

No fix is offered: the precise type depends on what callers pass.

### Active on

- **`object` in annotations of parameters, return values and variables**, including nested uses such as
  `dict[str, object]`: each one accepts any value. Both `object` and `builtins.object` match.

  - `value: object`: a parameter annotation.

    ```python report
    def describe(value: object) -> str:
        return str(value)
    ```

  - `-> object`: a return annotation.

    ```python report
    def make() -> object:
        return 1
    ```

  - `CACHE: dict[str, object]`: a variable annotation, with `object` nested in `dict`.

    ```python report
    CACHE: dict[str, object] = {}
    ```

- **`Any` in variable annotations and in nested positions**, such as `CACHE: Any`, `dict[str, Any]` or `-> list[Any]`:
  each one turns off type checking for the values it touches. Spelled `Any`, `typing.Any`, an import alias or
  `typing_extensions.Any`.

  - `CACHE: Any`: a variable annotation.

    ```python report
    from typing import Any

    CACHE: Any = {}
    ```

  - `dict[str, t.Any]`: a nested position, with `Any` spelled through the alias `t`.

    ```python report
    import typing as t

    CACHE: dict[str, t.Any] = {}
    ```

  - `-> list[Any]`: a nested position in a return annotation.

    ```python report
    from typing import Any


    def items() -> list[Any]:
        return []
    ```

### Exceptions

Parameters of these functions are exempt, since their signatures often have to accept any value. Their return
annotations are still checked.

- **Dunder functions and methods**, such as `__eq__(self, other: object)`: Python calls them with whatever value the
  operation received.

  ```python ok
  class Point:
      def __eq__(self, other: object) -> bool:
          return True
  ```

- **Functions that return [`TypeIs[...]`](https://docs.python.org/3/library/typing.html#typing.TypeIs)
  ([PEP 742](https://peps.python.org/pep-0742/)) or
  [`TypeGuard[...]`](https://docs.python.org/3/library/typing.html#typing.TypeGuard)
  ([PEP 647](https://peps.python.org/pep-0647/))**: they exist to narrow a value of unknown type.

  ```python ok
  from typing import TypeIs


  def is_text(value: object) -> TypeIs[str]:
      return isinstance(value, str)
  ```

- **A whole-annotation `Any` on a parameter or return**, such as `def f(value: Any)` or `-> Any`: Ruff's
  [any-type](https://docs.astral.sh/ruff/rules/any-type/) reports it, so this rule leaves it alone to avoid a double
  report.

  ```python ok
  from typing import Any


  def run(value: Any) -> None:
      print(value)
  ```

- **String annotations**: these are not checked.

  ```python ok
  def describe(value: "object") -> str:
      return str(value)
  ```

## Ruff overlap

Ruff's [any-type](https://docs.astral.sh/ruff/rules/any-type/) rejects `Any` as the whole annotation of a function
parameter or return value. This rule covers `object` everywhere, and `Any` in variable annotations and nested positions,
which [any-type](https://docs.astral.sh/ruff/rules/any-type/) misses.
