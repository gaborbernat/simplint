Flags imports of private names, private imports that reach beyond one direct relative module, imports that bind a
private module as an object, and public names imported under a private alias. In tests, imports from private modules
must match the test's path and name, unless they use an explicit package export.

## Why is this bad?

A private module can change without breaking callers when they import its public names through a stable path. For
example, `_a/__init__.py` can re-export `what` from `._x`. Callers then write `from ._a import what`, and moving `_x.py`
only requires changing the re-export. Importing `package._api` as an object exposes the implementation path to its
callers.

## Example

```python context file=package/__init__.py
```

```python context file=package/_api.py
def make() -> object:
    return object()
```

```python report file=package/consumer.py
import package._api as api

value = api.make()
```

Use instead:

```python ok file=package/consumer.py
from ._api import make

value = make()
```

## What it does

The rule checks imports in module, class and function bodies. It offers a fix only for a private alias (see Fix below):
choosing a public re-export or replacing uses of a module object may require changes elsewhere.

### Active on

- **Private module-object imports**, such as `import package._api as api` and `from . import _api`.

  ```python report
  import package._api as api


  def describe(value: api.API) -> None:
      print(value)
  ```

- **Absolute private imports**, such as `from package._api import API`. Use `from ._api import API` within the same
  package, or import `API` from a public re-export elsewhere.

  ```python report
  from package._api import API


  def describe(value: API) -> None:
      print(value)
  ```

- **Parent-relative private imports**, such as `from .._api import API` and `from .. import _api`.

  ```python report file=package/sub/consumer.py
  from .._api import API

  value = API()
  ```

- **Paths with two private names**, such as `from ._a._x import what` or `from ._internal._helpers import API`. A direct
  relative import may cross one private name. Re-export `what` in `_a/__init__.py` so the caller can import it from
  `._a`.

- **Private names from another module**, such as `from ._internal import _helper` or `from .public import _helper`. Give
  the name a public spelling inside its private module. A paired test follows the same rule.

  ```python report file=package/consumer.py
  from .public import _helper

  _helper()
  ```

  ```python context file=package/public.py
  def _helper() -> None: ...
  ```

  ```python report file=package/b.py
  from ._a._x import what

  what()
  ```

  ```python context file=package/_a/_x.py
  def what() -> None: ...
  ```

  Re-export `what` in the package's `__init__.py`, then import it from the package:

  ```python context file=package/_a/__init__.py
  from ._x import what
  ```

  ```python ok file=package/b.py
  from ._a import what

  what()
  ```

- **Private modules below a sibling**, such as `from ._api.helpers import API`.

  ```python report file=package/consumer.py
  from ._api.helpers import API

  value = API()
  ```

- **Unpaired test imports**, such as `tests/package/test_api.py` importing from `package._other`. A test of a private
  module may import from the matching module only; other behavior should be reached through public interfaces.

  ```python report file=tests/package/test_api.py
  from package._other import API


  def test_api(api: API) -> None:
      assert api
  ```

- **Public names under private aliases**, such as `from ._api import canonical as _canonical` or `import os as _os`. The
  alias hides that the module uses a public name, and a reader looking for `canonical` misses it. Import the name as
  itself.

  ```python report file=package/consumer.py
  from .windows import Windows as _Result

  VALUE = _Result
  ```

  ```python fixed file=package/consumer.py
  from .windows import Windows

  VALUE = Windows
  ```

  ```python context file=package/windows.py
  class Windows: ...
  ```

### Exceptions

- A package and its distribution's tests may import a **private subpackage's exports** when its `__all__` lists them by
  public name. For example, `pkg._native` can import `Settings` from `pkg._core` when `pkg/_core/__init__.py` lists it.
  Include the initializer in the check so simplint can read its export list. Platform branches may list different names.
  A computed list leaves the exports unknown.

- **A public name from a direct private child**: `from ._api import API` lets the package use a private sibling to build
  an API outside tests.

  ```python ok file=package/consumer.py
  from ._api import API

  value = API()
  ```

- **A paired test and private module**: `tests/package/test_api.py` may import names from `package._api`. The test
  directory may omit the source package's top-level name: `tests/_render/test_intersphinx.py` may import names from
  `package._render.intersphinx`. A colocated `package/test_api.py` may also import names from `._api`. For an absolute
  import, the remaining directories and the stem after `test_` must match an existing `.py` or `.pyi` file. A dedicated
  subtree may instead take the module's name: `tests/_html/url/test_url.py` may import names from `package._html` when
  `_html.py` or `_html.pyi` exists. The imported name must still be public.

  ```python context file=package/_api.py
  class API:
      pass
  ```

  ```python ok file=tests/package/test_api.py
  from package._api import API


  def test_api(api: API) -> None:
      assert api
  ```

  A relative import from the mirrored test tree resolves inside that tree, so it is not paired with the source module.
  Importing the private module as an object remains disallowed.

- **Dunder names**: `__future__` and `__version__` do not count as private names.

- **gettext's `_`**: `from ._i18n import _` imports the translation function by its conventional name.

- **`_typeshed`**: [typeshed](https://github.com/python/typeshed/tree/main/stdlib/_typeshed) documents it as the module
  stubs and `TYPE_CHECKING` blocks import types from, such as `from _typeshed import StrPath`.

  ```python ok file=consumer.py
  from __future__ import annotations

  from package.__version__ import __version__


  def show() -> str:
      return __version__
  ```

- **Documented public underscore names**: [`os._exit`](https://docs.python.org/3/library/os.html#os._exit) is a public
  API despite its underscore.

  ```python ok
  from os import _exit


  def stop() -> None:
      _exit(0)
  ```

### Fix

**Unsafe fix**, for a public name under a private alias: the fix drops the alias and renames the module's uses of it,
which breaks code elsewhere that reads the alias, such as a test patching `module._canonical`. No fix is offered when
the public name is already taken in the file, another file or a string reads the alias, the import is dotted
(`import a.b as _b`) or the import sits inside a function. Other findings may need a public name or a re-export, which
the author must choose.

## Ruff overlap

Ruff's [import-private-name](https://docs.astral.sh/ruff/rules/import-private-name/) (PLC2701, a preview rule) reports a
private import that a module or function uses at runtime or never uses, and this rule leaves those reports to it whether
or not the project selects PLC2701. It keeps the rest, such as imports used only in annotations or bound in a class
body, relative imports, imports within their own package, and `import package._api` without an alias, which Ruff does
not report. Ruff's [unconventional-import-alias](https://docs.astral.sh/ruff/rules/unconventional-import-alias/)
(ICN001) owns the aliases of the modules its import conventions name, such as `numpy`, so this rule leaves a private
alias of those to it.
