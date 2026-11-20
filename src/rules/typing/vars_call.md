Flags direct calls to the builtin `vars()`.

## Why is this bad?

For an instance with a `__dict__`, [typeshed](https://github.com/python/typeshed/blob/main/stdlib/builtins.pyi) types
`vars(obj)` as `dict[str, Any]`. A misspelled attribute used as a dictionary key can pass type checking. `vars(obj)`
[raises `TypeError`](https://docs.python.org/3/builtins/functions.html#vars) when the instance lacks `__dict__`.

Read named attributes when their types are known. For `argparse`, pass a
[`Namespace`](https://docs.python.org/3/library/argparse.html#argparse.Namespace) subclass with annotated attributes to
[`parse_args(namespace=...)`](https://docs.python.org/3/library/argparse.html#argparse.ArgumentParser.parse_args).

## Example

```python report
def options(parser: argparse.ArgumentParser) -> dict[str, str]:
    return vars(parser.parse_args())
```

Use instead:

```python ok
class Options(argparse.Namespace):
    output: str


def options(parser: argparse.ArgumentParser) -> Options:
    return parser.parse_args(namespace=Options())
```

## What it does

No fix is offered: the typed replacement depends on how the result is used.

### Active on

- **Calls to [`vars`](https://docs.python.org/3/builtins/functions.html#vars)**. `vars(obj)` reads the object's
  namespace. `vars()` without arguments reads the current locals.

  ```python report
  def names(obj: object) -> list[str]:
      return list(vars(obj))
  ```

### Exceptions

- **A call through a parameter or local variable named `vars`**: the name does not refer to the builtin there.

  ```python ok
  from collections.abc import Callable


  def read(vars: Callable[[str], str]) -> str:
      return vars("name")
  ```

- **Any call in a file that binds `vars` at module level**, for example by defining a function of that name: `vars`
  refers to that binding, not the builtin.

  ```python ok
  def vars(value: int) -> int:
      return value


  print(vars(1))
  ```

- **A class object**: `vars(cls)`, `vars(type(obj))`, `vars(obj.__class__)`, a class the file defines and a parameter
  annotated `type[...]`. `vars(cls)` returns a read-only mapping proxy containing the class's own attributes.

  ```python ok
  class Base:
      @classmethod
      def own_names(cls) -> list[str]:
          return list(vars(cls))
  ```

- **Passing `vars` as a value**: these are not reported.

  ```python ok
  items = [{"a": 1}]
  names = list(map(vars, items))
  ```

- **Calling `builtins.vars(...)`**: these are not reported.

  ```python ok
  import builtins

  values = builtins.vars(object())
  ```

- **A mapping whose keys come from `vars()` in the same expression**: `dict(vars(obj))`, a dict comprehension over
  `vars(obj).items()`, `vars(obj) | {...}` and `{**vars(obj), ...}` are not reported, since the attributes of an
  open-ended object have no key type to name. A `vars()` that only supplies a value, as in `{"attributes": vars(obj)}`,
  is still reported.

  ```python ok
  def copy(obj: object) -> dict[str, object]:
      return {**vars(obj), "prog": "x"}
  ```
