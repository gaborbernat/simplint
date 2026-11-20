Flags `object()` placeholders assigned to a name directly in a module or class body when the target is Python 3.15 or
later.

## Why is this bad?

A function signature that uses `object()` as a default displays an address, such as `<object object at 0x7f...>`.
Copying or pickling the object also breaks identity checks. With
[`MISSING = sentinel("MISSING")`](https://docs.python.org/3/builtins/functions.html#sentinel), the signature shows the
name, copies preserve identity, and `int | MISSING` is a valid type annotation. Pickling preserves identity when the
sentinel is reachable by its name in its defining module. [PEP 661](https://peps.python.org/pep-0661/) describes these
guarantees.

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

The rule checks a single-name assignment such as `MISSING = object()` directly in a module or class body. The name must
be bound only once in that scope. A binding in a method or nested class belongs to a different scope.

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

The fix is unsafe because it changes the representation and pickling behavior. For a class attribute it uses a qualified
name, such as `sentinel("Config.SENTINEL")`, so pickle can find the object through the module. A class defined inside a
function has no such path, so simplint reports it without a fix.

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
