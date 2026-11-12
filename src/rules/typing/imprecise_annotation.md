Flags `object` and `Any` in annotations, type alias values, and type parameter bounds and defaults. Both accept any
value, so the code behind them has to narrow the value at runtime, and a type checker stops checking every value they
touch.

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
  each one turns off type checking for the values it touches. Spelled `Any`,
  [`typing.Any`](https://docs.python.org/3/library/typing.html#typing.Any), an import alias or `typing_extensions.Any`.

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

- **`object` and `Any` in type alias values and in type parameter bounds and defaults**: a `type` statement and a type
  parameter name the type a value or parameter stands for, so the same rules apply to them.

  - `type Payload = object`: a type alias value.

    ```toml config
    [project]
    requires-python = ">=3.12"
    ```

    ```python report
    type Payload = object
    ```

  - `def first[T: Any](items: list[T]) -> T`: a type parameter bound.

    ```toml config
    [project]
    requires-python = ">=3.12"
    ```

    ```python report
    from typing import Any


    def first[T: Any](items: list[T]) -> T:
        return items[0]
    ```

  - `class Box[T = object]`: a type parameter default, from [PEP 696](https://peps.python.org/pep-0696/).

    ```toml config
    [project]
    requires-python = ">=3.13"
    ```

    ```python report
    class Box[T = object]:
        pass
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

- **`object` as the annotation of `*args` or `**kwargs`**: it is the honest type for arbitrary values, and ANN401 flags
  `Any` there unless `allow-star-arg-any` is set, so `object` is the one annotation both tools accept. A nested use such
  as `*args: list[object]` is still reported.

  ```python ok
  def forward(*args: object, **kwargs: object) -> None:
      print(args, kwargs)
  ```

- **An `Any` that ANN401 reports on a parameter or return**, such as `def f(value: Any)`, `-> Optional[Any]` or
  `def f(value: int | Any)`: Ruff's [ANN401](https://docs.astral.sh/ruff/rules/any-type/) reports it, so this rule
  leaves it alone to avoid a double report. ANN401 reports an `Any` that is the annotation, or sits in an `Optional`, a
  `Union`, a `|` or the first argument of `Annotated`.

  ```python ok
  from typing import Any


  def lookup(key: str) -> Any | None:
      return None
  ```

ANN401 skips the `self` or `cls` of a method, `@override` signatures, overload implementations and, with
`allow-star-arg-any`, `*args` and `**kwargs`. This rule still reports the `Any` in those, since no tool would report it
otherwise.

- **`object` as the return type of a `Callable`**, such as `Callable[..., object]` or `Callable[[int], object]`, from
  `typing` or [`collections.abc`](https://docs.python.org/3/library/collections.abc.html#module-collections.abc): the
  callable's result is ignored, which is what `object` says there. `object` in its parameter types is still reported.

  ```python ok
  from collections.abc import Callable


  def run(callback: Callable[[int], object]) -> None:
      callback(1)
  ```

- **`object` values in the metadata of `Annotated`**, such as `object()` in `Annotated[int, object()]`: only the first
  argument of `Annotated` is an annotation, so an `object` after it is a value and is not checked. `object` as the first
  argument is still reported, and so is an `Any` in the metadata, which ANN401 does not check.

  ```python ok
  from typing import Annotated


  def pick(value: Annotated[int, object()]) -> int:
      return value
  ```

  ```python report
  from typing import Annotated


  def first(value: Annotated[object, "meta"]) -> int:
      return 1
  ```

  ```python report
  from typing import Annotated, Any


  def meta(value: Annotated[int, {"type": Any}]) -> int:
      return value
  ```

- **String annotations**: these are not checked.

  ```python ok
  def describe(value: "object") -> str:
      return str(value)
  ```

## Ruff overlap

Each tool owns a different part of the `Any` reports:

- **[ANN401](https://docs.astral.sh/ruff/rules/any-type/) owns the `Any` it reports** on a parameter or return value:
  the whole annotation, or an `Any` inside `Optional`, `Union`, `|` or `Annotated`. It reports each such annotation
  once, at its first character, so `Any | None` and `Optional[Any]` are one report each, at the start of the annotation,
  not at the `Any`.
- **This rule owns everything else**: `object` everywhere except as the annotation of `*args` or `**kwargs`, `Any` in
  variable annotations, and `Any` in other nested positions such as `list[Any]`, which ANN401 does not report. It also
  keeps the `Any` of the signatures ANN401 skips, such as `self: Any` or an `@override` method.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rule
  [ANN401](https://docs.astral.sh/ruff/rules/any-type/), which reports a parameter or return annotation that is `Any` or
  a union with `Any` and which this rule leaves to it; this rule covers `object` and `Any` nested in other types or in
  variables (open issue [#5871](https://github.com/astral-sh/ruff/issues/5871)).
- [basedpyright](https://github.com/DetachHead/basedpyright) reports every explicit `Any` as
  [reportExplicitAny](https://docs.basedpyright.com/latest/configuration/config-files/). It does not report `object`,
  which this rule also flags.
