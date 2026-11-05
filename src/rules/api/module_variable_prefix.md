Flags module-scope variables whose leading underscore does not match their visibility. A name that no other module
imports and `__all__` does not list needs one, and a name that `__all__` lists must not have one.
[module-variable-case](SMP102.html) checks the case.

## Why is this bad?

A leading underscore tells readers whether other modules may use a name
([PEP 8](https://peps.python.org/pep-0008/#public-and-internal-interfaces)). Deriving it from `__all__` and from the
imports of other modules keeps the underscore in line with what the module exports.

The expected underscore comes from two questions, asked in this order:

1. **Another module in the run imports it**: keep its leading underscore as it is.
1. **`__all__` lists it**: no leading underscore.
1. **Neither**: a leading underscore. In a module without `__all__`, every variable counts as private.

## Example

```python report
LIMIT = 3
_LOGGER = logging.getLogger(__name__)

__all__ = ["_LOGGER"]
```

Use instead:

```python ok
_LIMIT = 3
LOGGER = logging.getLogger(__name__)

__all__ = ["LOGGER"]
```

## Configuration

- [`naming.keep-names`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#naming) in
  `[tool.simplint]`: module-scope names the rule leaves as they are, such as `keep-names = ["limit"]` under
  `[tool.simplint.naming]`

  ```toml config
  [tool.simplint.naming]
  keep-names = ["limit"]
  ```

  ```python ok
  limit = 3
  ```

## What it does

### Active on

- **Names listed in [`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package)**: they have
  no leading underscore, as the module exports them.

  ```python report
  _limit = 3

  __all__ = ["_limit"]
  ```

- **Names neither listed in `__all__` nor imported by another module**: they have a leading underscore. In a module
  without `__all__`, every variable counts as private.

  ```python report
  LIMIT = 3
  ```

simplint reads `__all__` from assignments, `+=`, `.extend([...])` and `.append("name")` with string literals.

### Exceptions

- **Names another module in the run imports**: the importer already depends on the name, so its underscore stays as it
  is.

  ```python ok file=config.py
  limit = 3
  ```

  ```python context file=consumer.py
  from config import limit

  print(limit)
  ```

- **Names [module-variable-case](SMP102.html) leaves alone**: the same exceptions apply, since renaming these breaks the
  program or the tool that reads them by their exact spelling. They include dunder names, class-like names, type aliases
  from a `type` statement, names also bound by an import, `def` or `class`, names bound only as loop targets, the
  framework names and `naming.keep-names`.

  ```python ok
  Alias = int
  ```

  ```toml config
  [project]
  requires-python = ">=3.12"
  ```

  ```python ok
  type vector = list[float]
  ```

  ```python ok
  from typing import TYPE_CHECKING

  if TYPE_CHECKING:
      pass
  ```

  ```python ok file=tests/test_api.py
  import pytest

  pytestmark = pytest.mark.slow
  ```

### Fix

The fix adds the leading underscore, or removes every leading underscore, and keeps the rest of the name as written. It
renames every use in the file, including the `__all__` entry. **Unsafe fix:** always, as importers outside the run keep
the old name, and so do string references in unchecked files.

Removing every underscore, rather than one, keeps `__x` from becoming `_x`, which the rule would report again.

```python report
__all__ = ["_LIMIT"]

_LIMIT = 3
```

```python fixed
__all__ = ["LIMIT"]

LIMIT = 3
```

```python report
limit = 3
print(limit)
```

```python fixed
_limit = 3
print(_limit)
```

No fix is offered in the cases [module-variable-case](SMP102.html) lists under its fix, such as an importer outside the
run or a string that holds the old name.

### Related rules

[module-variable-case](SMP102.html) checks the case of the same variables. [unexported-public-name](SMP403.html) applies
the underscore convention to functions and classes.
