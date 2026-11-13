Flags imports of third-party modules that no requirement or extra in `[project]` installs, in the modules that run when
the package is imported.

## Why is this bad?

An import of a module the project does not declare works on the author's machine, where a development tool or another
package happened to install it, and fails with `ModuleNotFoundError` for a user who installs only what the project
lists. A module another requirement pulls in is no safer: that requirement can drop it in any release.

## Example

```toml config
[project]
name = "app"
dependencies = ["requests"]
```

```python context file=app/__init__.py
from app import client
```

```python report file=app/client.py
import requests
import yaml
```

Use instead:

```toml config
[project]
name = "app"
dependencies = ["requests", "pyyaml"]
```

```python context file=app/__init__.py
from app import client
```

```python ok file=app/client.py
import requests
import yaml
```

## What it does

The rule reads the modules that importing a top-level package of the project, or running its `__main__.py`, loads
through module-level imports. In each it checks the absolute imports that run when the module loads against the
`dependencies` and `optional-dependencies` of `[project]`. A distribution matches the modules named after it, such as
`google.auth` for `google-auth`, the modules whose name it contains, such as `serial` for `pyserial`, and a few known
renames, such as `yaml` for `pyyaml`.

### Exceptions

- **Projects whose `[project]` leaves `dependencies` to the build backend** through `dynamic`, or that have no
  `[project]` table: the rule cannot see their requirements.

- **Modules the package never imports itself**, such as a pytest plugin, a tool's hook or an optional integration a user
  imports by name, and the tests and scripts beside the package: they run where their user installed what they import.

  ```toml config
  [project]
  name = "app"
  dependencies = []
  ```

  ```python context file=app/__init__.py
  ```

  ```python ok file=app/pytest_plugin.py
  import pytest
  ```

- **Optional and deferred imports**: an import in a `try` that handles `ImportError`, and the fallback in its handler,
  one under `if TYPE_CHECKING:`, one under a check of `sys.version_info`, `sys.platform` or `os.name`, and one inside a
  function, which runs only when called.

  ```toml config
  [project]
  name = "app"
  dependencies = []
  ```

  ```python context file=app/__init__.py
  from app import render
  ```

  ```python ok file=app/render.py
  try:
      import rich
  except ImportError:
      rich = None


  def plot() -> None:
      import matplotlib

      print(matplotlib, rich)
  ```

- **The standard library of any Python version**, so `tomllib` behind a version check is not a requirement, and modules
  of the run, such as a second top-level package the distribution ships.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** no rule.
- [deptry](https://github.com/fpgmaas/deptry) reports the same imports as DEP001 (missing) and DEP003 (transitive),
  across every module of the project, deferred and guarded imports included, with its own name mapping read from the
  installed distributions.
- [Pylint](https://github.com/pylint-dev/pylint)'s `import-error` (E0401) reports a module the environment it runs in
  cannot import, so it misses a module some other package installed.
