Flags imports of third-party modules that no requirement or extra in `[project]` installs, in the modules that run when
the package is imported.

## Why is this bad?

An undeclared import may work locally because a development tool installed the package. A user installing only the
declared requirements can then get a
[`ModuleNotFoundError`](https://docs.python.org/3/builtins/exceptions.html#ModuleNotFoundError). A transitive dependency
can also disappear when the package that installed it changes its requirements.

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

simplint follows runtime imports from top-level package initializers and every non-test `__main__.py`, including those
in nested packages. It checks absolute imports in the reachable non-test modules of the shipped packages against
`project.dependencies` and `project.optional-dependencies`. A distribution matches the modules named after it, such as
`google.auth` for `google-auth`, the modules whose name it contains, such as `serial` for `pyserial`, and a few known
renames, such as `yaml` for `pyyaml`.

### Exceptions

- **Projects without `project.name` or with `dependencies` in `project.dynamic`**. The rule needs static project
  metadata.

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

- **Function bodies and import guards**. simplint skips imports inside functions, `if TYPE_CHECKING:` bodies and the
  body and handlers of a `try` that catches an import error. Bare handlers and handlers for
  [`Exception`](https://docs.python.org/3/builtins/exceptions.html#Exception) or
  [`BaseException`](https://docs.python.org/3/builtins/exceptions.html#BaseException) also qualify. The rule still
  checks imports in `else` and `finally` clauses of that `try`.

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

- **Environment checks**. simplint skips an entire `if` statement when its initial condition references
  [`sys.version_info`](https://docs.python.org/3/library/sys.html#sys.version_info),
  [`sys.platform`](https://docs.python.org/3/library/sys.html#sys.platform),
  [`sys.implementation`](https://docs.python.org/3/library/sys.html#sys.implementation),
  [`os.name`](https://docs.python.org/3/library/os.html#os.name) or an attribute of `platform`.

- **Modules whose top-level name belongs to the standard library in any checked version from Python 3.7 through 3.15**,
  so `tomllib` behind a version check is not a requirement, and modules of the run, such as a second top-level package
  the distribution ships.

## Other tools

- [deptry](https://deptry.com/rules-violations/) distinguishes missing dependencies (DEP001) from transitive
  dependencies (DEP003). It scans imports in the files selected for its run and uses installed distributions to help
  match imports to dependencies.
- [Pylint](https://github.com/pylint-dev/pylint)'s `import-error` (E0401) checks whether it can locate the imported
  module. A transitive dependency that is already installed passes that check.
