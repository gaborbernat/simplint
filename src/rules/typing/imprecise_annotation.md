Flags `object` and `Any` in annotations, type alias values, and type parameter bounds and defaults.

## Why is this bad?

`object` accepts any value. To use a type-specific operation you must narrow the type or tell the checker which type to
assume. `Any` permits unchecked operations and can
[propagate to other expressions](https://mypy.readthedocs.io/en/stable/dynamic_typing.html#operations-on-any-values). A
more specific annotation lets the checker catch incorrect calls.

The [typing guidelines](https://typing.python.org/en/latest/reference/best_practices.html#arguments-and-return-types)
recommend `object` for parameters that accept every value. simplint still reports it outside the exceptions below.

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
  `typing` or [`collections.abc`](https://docs.python.org/3/library/collections.abc.html#module-collections.abc):
  callers can accept a result of any type, for example when they discard it. `object` in its parameter types is still
  reported.

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

Ruff's [ANN401](https://docs.astral.sh/ruff/rules/any-type/) reports the whole parameter or return annotation when it
finds `Any` directly, in a union, in `Optional` or in the type argument of `Annotated`. simplint skips those
occurrences. It can still report `Any` in variables, in other nested types, and in signatures ANN401 skips. The
exceptions above apply to `object` independently.

## Other tools

- [basedpyright](https://github.com/DetachHead/basedpyright) reports every explicit `Any` as
  [reportExplicitAny](https://docs.basedpyright.com/latest/configuration/config-files/). It does not report `object`,
  which this rule also flags.
