Flags private imports that reach beyond one direct relative module, and imports that bind a private module as an object.
In tests, imports from private modules must match the test's path and name.

## Why is this bad?

Importing a module such as `package._api` exposes its implementation path to callers. Importing names directly lets a
package keep its implementation private while choosing what to expose through `__all__`. A parent-relative import such
as `from .._api import API` also crosses a package boundary.

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

The rule checks imports in module, class and function bodies. It offers no fix for an import alone: choosing a public
re-export or replacing uses of a module object may require changes elsewhere.

### Active on

- **Private module-object imports**, such as `import package._api as api` and `from . import _api`.

- **Absolute private imports**, such as `from package._api import API`. Use `from ._api import API` within the same
  package, or import `API` from a public re-export elsewhere.

- **Parent-relative private imports**, such as `from .._api import API` and `from .. import _api`.

- **Paths with two private names**, such as `from ._internal._helpers import API` or `from ._internal import _helper`. A
  direct relative import may cross one private name, not two.

- **Private modules below a sibling**, such as `from ._api.helpers import API`.

- **Unpaired test imports**, such as `tests/package/test_api.py` importing from `package._other`. A test of a private
  module may import from the matching module only; other behavior should be reached through public interfaces.

### Exceptions

- **One private name in a direct relative import**: `from ._api import API` lets the package use a private sibling to
  build an API outside tests. `from .public import _helper` also stays within the same package level.

- **A paired test and private module**: `tests/package/test_api.py` may import names from `package._api`. The test
  directory may omit the source package's top-level name: `tests/_render/test_intersphinx.py` may import names from
  `package._render.intersphinx`. A colocated `package/test_api.py` may also import names from `._api`. The remaining
  directories and the stem after `test_` must match an existing source file. A dedicated subtree may instead take the
  module's name: `tests/_html/url/test_url.py` may import names from `package._html` when `_html.py` or `_html.pyi`
  exists.

  A relative import from the mirrored test tree resolves inside that tree, so it is not paired with the source module.
  Importing the private module as an object remains disallowed.

- **Dunder names**: `__future__` and `__version__` do not count as private names.

- **Documented public underscore names**: [`os._exit`](https://docs.python.org/3/library/os.html#os._exit) is a public
  API despite its underscore.

## Ruff overlap

Ruff's [import-private-name](https://docs.astral.sh/ruff/rules/import-private-name/) (PLC2701, a preview rule) reports a
private import that a module or function uses at runtime or never uses, and this rule leaves those reports to it whether
or not the project selects PLC2701. It keeps the rest, such as imports used only in annotations or bound in a class
body, relative imports, imports within their own package, and `import package._api` without an alias, which Ruff does
not report.

## Other tools

[Ruff's import-private-name](https://docs.astral.sh/ruff/rules/import-private-name/) (PLC2701) reports private imports
across packages, but exempts relative imports and absolute imports within the same top-level package. This rule checks
those paths and module-object imports too.
