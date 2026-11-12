Flags direct calls to the builtin `vars()`, which return an untyped `dict`.

## Why is this bad?

`vars()` returns a `dict[str, Any]` in [typeshed](https://github.com/python/typeshed/blob/main/stdlib/builtins.pyi), so
the attribute types are lost and a misspelled key fails only at runtime. It also
[raises `TypeError`](https://docs.python.org/3/builtins/functions.html#vars) for an object without a `__dict__`, such as
one whose class defines `__slots__`. Reading attributes from a typed object keeps these checks in the type checker. For
[`argparse`](https://docs.python.org/3/library/argparse.html), pass a
[`Namespace`](https://docs.python.org/3/library/argparse.html#argparse.Namespace) subclass with annotated attributes to
[`parse_args(namespace=...)`](https://docs.python.org/3/library/argparse.html#argparse.ArgumentParser.parse_args), as in
the example. The [argparse docs](https://docs.python.org/3/library/argparse.html#the-namespace-object) show `vars(args)`
for a dict view. The typed form keeps the checks that an untyped dict loses, so the rule reports the call.

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

- **Direct calls to the builtin [`vars()`](https://docs.python.org/3/builtins/functions.html#vars)**: each one turns a
  typed object into an untyped `dict`.

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

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issue [#3845](https://github.com/astral-sh/ruff/issues/3845).
