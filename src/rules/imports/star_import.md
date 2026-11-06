Flags `from module import *` when the project defines `module`, and fixes it to import the names the file uses.

## Why is this bad?

A star import binds every public name of the module, so a reader cannot tell where a name comes from. A name the module
adds later can shadow one the file defines, and a linter cannot tell a name that is undefined from one the star import
provides. When the module lists its exports, the file can name the ones it uses and leave the rest out.

In the example below, `app.py` reads `area` from `geometry.py`, but the reader has to open `geometry.py` to learn that.

## Example

```python context file=geometry.py
__all__ = ["area", "perimeter"]


def area(side):
    return side * side


def perimeter(side):
    return 4 * side
```

```python report file=app.py
from geometry import *

print(area(2))
```

```python fixed file=app.py
from geometry import area

print(area(2))
```

## Conflicts with Ruff

Ruff's [F403](https://docs.astral.sh/ruff/rules/undefined-local-with-import-star/) reports the star import and
[F405](https://docs.astral.sh/ruff/rules/undefined-local-with-import-star-usage/) reports each name it cannot place,
both without a fix. Neither rule contradicts this one, so both may stay on; F405 goes quiet once the import names the
file's names.

## What it does

simplint looks the module up in the index of the run, so a star import of a module outside the project stays as it is.

### Active on

- **`from module import *` at module level**, including one inside a top-level `if` or `try` block, of a module the run
  lints, when the file reads a name the module defines or lists, or lists one in its own `__all__`.

  ```python context file=geometry.py
  VALUE = 1
  ```

  ```python report file=app.py
  from geometry import *

  print(VALUE)
  ```

### Exceptions

- **Star imports of modules outside the project**, such as the standard library, which the index does not know.

  ```python ok file=app.py
  from os.path import *
  ```

- **Re-exports**: a star import the file reads nothing from and whose names its own `__all__` does not list, which is
  there to pass the module's names on, and any star import of a module whose own `__all__` is computed, often from the
  imported module's.

  ```python context file=geometry.py
  VALUE = 1
  ```

  ```python ok file=shapes.py
  from geometry import *
  ```

- **Fallbacks in an `except` clause**, which must take the same names as the import the `try` attempts, as when a pure
  Python module stands in for a C accelerator.

  ```python context file=_pygeometry.py
  def area(side):
      return side * side
  ```

  ```python ok file=geometry.py
  try:
      from _geometry import *
  except ImportError:
      from _pygeometry import *
  ```

### Fix

The fix replaces the `*` with the names the file reads from the module, in alphabetical order. It applies only to a file
with one star import and no other file star-imports it.

A name is kept when the file reads it, that is it uses it and no binding of the file or builtin defines it, or when the
file lists it in its own `__all__`. The fix declines when other files or a library's users reach a name through this
file that the file does not read, since the import it would need is unused, and Ruff's F401 removes unused imports.

The fix is safe when the module lists its exports in a static `__all__`, since then every name the star import could
provide is known. Without an `__all__`, the module's public names are only those it defines, and the module may export
names it imports, so the fix is unsafe. Either way it declines when the file reads a name the module does not list or
define, as that name may come from one of the module's imports, or from an `__all__.append()` inside an `if` block that
simplint cannot read:

```python context file=geometry.py
import sys

__all__ = ["area"]


def area(side):
    return side * side


if sys.platform == "win32":

    def native_area(side):
        return side * side

    __all__.append("native_area")
```

```python report file=app.py
import sys

from geometry import *

print(area(2))
if sys.platform == "win32":
    print(native_area(2))
```

```python context file=helpers.py
def double(value):
    return 2 * value
```

```python report file=tools.py
from helpers import *

print(double(2))
```

```python fixed file=tools.py
from helpers import double

print(double(2))
```

The fix stays off when the module's exports cannot be read statically, as for a dynamic `__all__` or a module-level
`__getattr__`, and when a comment sits on the import or on the line above it, since the comment may say why the star
import is there. It also stays off in an `__init__.py` without a static `__all__` of its own: there the star import
makes up the package's names, which its users may reach as `package.name`.

```python context file=geometry.py
__all__ = ["area", "perimeter"]


def area(side):
    return side * side


def perimeter(side):
    return 4 * side
```

```python report file=shapes/__init__.py
from geometry import *

UNIT = area(1)
```

### Undetected cases

A star import whose module lives outside the project is not reported, since its names are unknown. Neither is one whose
module defines the names the file reads in a way simplint cannot see, as through `globals()` or `exec`, since the file
then reads nothing the module is known to define.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** released rule
  [F403](https://docs.astral.sh/ruff/rules/undefined-local-with-import-star/).
- [pylint](https://github.com/pylint-dev/pylint) reports every
  [wildcard-import](https://pylint.readthedocs.io/en/latest/user_guide/messages/warning/wildcard-import.html), including
  modules outside the project. This rule reports only modules the project defines, and its fix names the imported names.
- [autoflake](https://github.com/PyCQA/autoflake) expands the same star imports with
  [`--expand-star-imports`](https://github.com/PyCQA/autoflake), and it only applies to files with a single star import.
