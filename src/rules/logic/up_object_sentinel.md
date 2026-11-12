Flags a module or class constant assigned a bare [`object()`](https://docs.python.org/3/builtins/functions.html#object)
placeholder, which [`sentinel`](https://docs.python.org/3/builtins/functions.html#sentinel) from Python 3.15 names
([PEP 661](https://peps.python.org/pep-0661/)).

## Why is this bad?

A bare `object()` reprs as an address, so a traceback or a debugger shows `<object object at 0x7f...>` where the name
would tell the reader which placeholder it is. `sentinel` takes the name and uses it in the repr.

## Example

```toml config
[project]
requires-python = ">=3.15"
```

```python report
_MISSING = object()
```

Use instead:

```python ok
_MISSING = sentinel("_MISSING")
```

## Configuration

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python);
  see [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

  ```toml config
  [project]
  requires-python = ">=3.14"
  ```

  ```python ok
  _MISSING = object()
  ```

## What it does

Reports a single-name assignment of a bare `object()` call, with no arguments, when it sits in a module body or a class
body and the name is assigned only once in that body. Assignments in a method, a nested class or a comprehension of the
class do not count, nor does a name of the same spelling in another class or the module.

### Active on

```toml config
[project]
requires-python = ">=3.15"
```

```python report
class Config:
    SENTINEL = object()
```

```python fixed
class Config:
    SENTINEL = sentinel("Config.SENTINEL")
```

### Exceptions

- **A name assigned again**: the second binding is a different object, so the placeholder is not the only one.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python ok
  _MISSING = object()
  _MISSING = 2
  ```

- **A class attribute assigned again in its class**: the class body holds two objects under the name.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python ok
  class Config:
      MISSING = object()
      MISSING = None
  ```

- **A placeholder in a function**: a local is not visible by name outside the function.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python ok
  def run():
      missing = object()
      return missing
  ```

- **A file that already spells `sentinel`**: the name may be a different binding.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python ok
  sentinel = 1
  _MISSING = object()
  ```

- **Target versions before 3.15**: the builtin does not exist there.

  ```toml config
  [project]
  requires-python = ">=3.14"
  ```

  ```python ok
  _MISSING = object()
  ```

### Fix

The fix is unsafe. The repr changes from an address to the name, which a program can read, and a pickled `sentinel`
loads only from the attribute it is stored under, so a class attribute is named with its classes, as
`sentinel("Config.SENTINEL")`. A placeholder in a class inside a function is reported without a fix, since no qualified
name reaches it.

```toml config
[project]
requires-python = ">=3.15"
```

```python report
def build():
    class Config:
        SENTINEL = object()

    return Config
```

```python fixed
def build():
    class Config:
        SENTINEL = object()

    return Config
```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
