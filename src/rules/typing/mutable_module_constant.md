Flags lists, sets and dictionaries assigned to module constants when simplint finds only read access. The fix changes
the collection type.

## Why is this bad?

A caller can append to a module-level list and change what other callers see.
[`Final`](https://peps.python.org/pep-0591/#specification) prevents reassignment during type checking. To prevent
changes to the collection itself, use a tuple, a frozenset or a read-only mapping. Python 3.15 added
[`frozendict`](https://peps.python.org/pep-0814/), which is hashable when all its values are hashable. On earlier
versions the fix uses [`MappingProxyType`](https://docs.python.org/3/library/types.html#types.MappingProxyType).

## Example

```python report
SUFFIXES = [".py", ".pyi"]


def is_source(name: str) -> bool:
    return name.endswith(tuple(SUFFIXES))
```

Use instead:

```python ok
SUFFIXES = (".py", ".pyi")


def is_source(name: str) -> bool:
    return name.endswith(SUFFIXES)
```

## Configuration

- The Python version the code targets, which picks `frozendict` from Python 3.15 and `MappingProxyType` before it; see
  [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

  ```toml config
  [tool.ruff]
  target-version = "py315"
  ```

  ```python report
  CODES = {"ok": 200}


  def status(name: str) -> int:
      return CODES[name]
  ```

  ```python fixed
  CODES = frozendict({"ok": 200})


  def status(name: str) -> int:
      return CODES[name]
  ```

## What it does

### Active on

A module-level assignment to one name, which is `UPPER_CASE` or annotated as `Final`, of a list, set or dict display, a
comprehension, or a call to the `list`, `set` or `dict` built-in. Every use of the name in the module only reads the
value: it subscripts it, iterates it, tests it, compares it, calls a method that leaves it as it is, such as `get` or
`items`, or passes it to a built-in that reads or copies it, such as `len` or `sorted`.

- **An `UPPER_CASE` name**: leading underscores included.

  ```python report
  _ALIASES = {"py": "python"}


  def language(suffix: str) -> str:
      return _ALIASES.get(suffix, suffix)
  ```

- **A name annotated as `Final`**: whatever its case.

  ```python report
  from typing import Final

  defaults: Final = {"retries": 3}


  def retries() -> int:
      return defaults["retries"]
  ```

- **A comprehension or a built-in call**: both build a new collection.

  ```python report
  SQUARES = [value * value for value in range(10)]
  ```

### Exceptions

- **A constant some code changes or hands out**: a method that changes it, a subscript assignment, a `del`, or passing
  it to a function or returning it, which may hand it to code that changes it.

  ```python ok
  REGISTRY = {}


  def register(name: str, value: int) -> None:
      REGISTRY[name] = value
  ```

  ```python ok
  NAMES = ["a", "b"]


  def names() -> list[str]:
      return NAMES
  ```

- **Names used or written by another module in the run**. Another module may mutate the collection or replace it. A star
  import or a write through a computed attribute name can affect any name in the module. Writes through
  [`globals()`](https://docs.python.org/3/builtins/functions.html#globals) in the defining module count too.

  ```python ok file=app/settings.py
  PLUGINS = ["core"]
  ```

  ```python ok file=app/main.py
  from app.settings import PLUGINS

  PLUGINS.append("extra")
  ```

- **A Sphinx `conf.py`**: Sphinx hands its settings to extensions, some of which change them, as `sphinx-copybutton`
  updates `html_context`.

- **A name assigned more than once**: it is a variable, not a constant.

  ```python ok
  LEVELS = ["info"]
  LEVELS = ["debug", *LEVELS]
  ```

- **Names outside the convention**. simplint skips lowercase names without `Final`, dunder names and tool-defined names
  such as `pytestmark`.

  ```python ok
  handlers = ["console"]
  __all__ = ["handlers"]
  ```

### Fix

| Value                                                                                             | Fix                                                                             | Safe |
| ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | :--: |
| list display                                                                                      | a tuple display, with a trailing comma for one element                          |  ✗   |
| list comprehension                                                                                | wrapped in `tuple(...)`                                                         |  ✗   |
| set display or comprehension                                                                      | wrapped in `frozenset(...)`                                                     |  ✗   |
| dict display or comprehension, Python 3.15+                                                       | wrapped in `frozendict(...)`                                                    |  ✗   |
| dict display or comprehension, before 3.15                                                        | wrapped in `MappingProxyType(...)`, importing it from `types`                   |  ✗   |
| `list(...)`, `set(...)` or `dict(...)` call                                                       | calls `tuple`, `frozenset` or `frozendict`, or wraps `dict(...)` before 3.15    |  ✗   |
| annotation `list[T]`, `set[T]`, `dict[K,V]`                                                       | becomes `tuple[T, ...]`, `frozenset[T]` or the mapping type, inside `Final` too |  ✗   |
| annotation [`typing.List`](https://docs.python.org/3/library/typing.html#typing.List) or a string | no fix                                                                          |      |
| a read that sees the type, listed below                                                           | no fix                                                                          |      |

simplint withholds the fix when the module uses the collection in any of these ways.

- **A list compared**: a tuple never equals a list, and ordering the two raises
  [`TypeError`](https://docs.python.org/3/builtins/exceptions.html#TypeError). Being the container of an `in` test is
  fine.
- **A binary operator** can change the result type or fail after conversion. For example, a tuple cannot be concatenated
  with a list, and a union with a frozenset as its left operand produces a frozenset.
- **`copy()` on a list**: a tuple has no `copy` method.
- **`isinstance`, `type`, `repr`, `str` or `print`**: these calls can reveal the change of type.

```python report
LEGACY = ["==2.1.0.3"]


def is_legacy(specifier: str) -> bool:
    return specifier in LEGACY + ["<1.0"]
```

The fix is unsafe because callers can rely on the original collection type. For example, slicing a list returns a list,
while slicing a tuple returns a tuple. An abstract annotation such as `Sequence[str]` can describe either type and stays
in place.

```python report
PORTS: dict[str, int] = {"http": 80}


def port(name: str) -> int:
    return PORTS[name]
```

```python fixed
from types import MappingProxyType

PORTS: MappingProxyType[str, int] = MappingProxyType({"http": 80})


def port(name: str) -> int:
    return PORTS[name]
```

```python report
SINGLE: list[str] = ["only"]
```

```python fixed
SINGLE: tuple[str, ...] = ("only",)
```

## Ruff overlap

[Ruff RUF012](https://docs.astral.sh/ruff/rules/mutable-class-default/) checks mutable class attributes.
[C408](https://docs.astral.sh/ruff/rules/unnecessary-collection-call/) can rewrite `dict(a=1)` as `{"a": 1}`. simplint
can freeze the resulting module constant.

## Other tools

- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports a mutable module
  constant as
  [WPS407](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/best_practices.html#wemake_python_styleguide.violations.best_practices.MutableModuleConstantViolation),
  whether or not code changes it, with no fix.
