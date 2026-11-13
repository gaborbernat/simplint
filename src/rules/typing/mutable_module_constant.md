Flags a module constant bound to a list, set or dict that no code changes, and makes it a tuple, a frozenset or a
read-only mapping.

## Why is this bad?

A module constant is shared by every caller, and a mutable one can be changed by any of them: a function that appends to
it changes what every other caller reads, for the rest of the process. `Final`
([PEP 591](https://peps.python.org/pep-0591/#specification)) stops rebinding the name, not changing the value it holds.
An immutable value states that the constant does not change, and the runtime enforces it: a
[`tuple`](https://docs.python.org/3/library/stdtypes.html#tuple) and a
[`frozenset`](https://docs.python.org/3/library/stdtypes.html#frozenset) have no methods that change them, and
[`frozendict`](https://docs.python.org/3.15/library/stdtypes.html#frozendict), added in Python 3.15 by
[PEP 814](https://peps.python.org/pep-0814/), or a
[`MappingProxyType`](https://docs.python.org/3/library/types.html#types.MappingProxyType) before it, raises on
assignment. A frozenset and a frozendict are also hashable, so the constant can serve as a dict key or a cache argument.

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

- **A name another module imports**: that module may change the value, which this module cannot see.

  ```python ok file=app/settings.py
  PLUGINS = ["core"]
  ```

  ```python ok file=app/main.py
  from app.settings import PLUGINS

  PLUGINS.append("extra")
  ```

- **A Sphinx `conf.py`**: Sphinx hands its settings to extensions, some of which change them, as
  `sphinx-copybutton` updates `html_context`.

- **A name assigned more than once**: it is a variable, not a constant.

  ```python ok
  LEVELS = ["info"]
  LEVELS = ["debug", *LEVELS]
  ```

- **A lowercase name without `Final`, and a dunder**: neither reads as a constant.

  ```python ok
  handlers = ["console"]
  __all__ = ["handlers"]
  ```

### Fix

| Value                                       | Fix                                                                             | Safe |
| ------------------------------------------- | ------------------------------------------------------------------------------- | :--: |
| list display                                | a tuple display, with a trailing comma for one element                          |  ✗   |
| list comprehension                          | wrapped in `tuple(...)`                                                         |  ✗   |
| set display or comprehension                | wrapped in `frozenset(...)`                                                     |  ✗   |
| dict display or comprehension, Python 3.15+ | wrapped in `frozendict(...)`                                                    |  ✗   |
| dict display or comprehension, before 3.15  | wrapped in `MappingProxyType(...)`, importing it from `types`                   |  ✗   |
| `list(...)`, `set(...)` or `dict(...)` call | calls `tuple`, `frozenset` or `frozendict`, or wraps `dict(...)` before 3.15    |  ✗   |
| annotation `list[T]`, `set[T]`, `dict[K,V]` | becomes `tuple[T, ...]`, `frozenset[T]` or the mapping type, inside `Final` too |  ✗   |
| annotation `typing.List` or a string        | no fix                                                                          |      |
| a read that sees the type, listed below     | no fix                                                                          |      |

A read in the module that sees the type leaves the fix off, as the immutable value would change what it does:

- **A list compared**: a tuple never equals a list, and ordering the two raises `TypeError`. Being the container of an
  `in` test is fine.
- **An operator**: `+` raises between a tuple and a list, and `|` on a `frozenset` or a mapping view returns a
  `frozenset` or a `dict` where a `set` or the view came back before.
- **`copy()` on a list**: a tuple has no `copy` method.
- **`isinstance`, `type`, `repr`, `str` or `print`**: each shows the new type.

```python report
LEGACY = ["==2.1.0.3"]


def is_legacy(specifier: str) -> bool:
    return specifier in LEGACY + ["<1.0"]
```

The fix is still unsafe: code the module cannot see, such as a caller that compares what a function built from the
value, may rely on the list, set or dict API. An abstract annotation, such as `Sequence[str]`, already fits the
immutable value and stays.

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

Ruff's [mutable-class-default](https://docs.astral.sh/ruff/rules/mutable-class-default/) (RUF012) reports mutable class
attributes, not module constants. Ruff's
[unnecessary-collection-call](https://docs.astral.sh/ruff/rules/unnecessary-collection-call/) (C408) rewrites
`dict(a=1)` as `{"a": 1}`; this rule's fix then wraps the display, so the two compose.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not implemented; requested in a comment on
  [#3845](https://github.com/astral-sh/ruff/issues/3845).
- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports a mutable module
  constant as
  [WPS407](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/best_practices.html#wemake_python_styleguide.violations.best_practices.MutableModuleConstantViolation),
  whether or not code changes it, with no fix.
