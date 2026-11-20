Flags `from ... import` statements that name the file's own package, or a package below it, by its absolute path.

## Why is this bad?

A [relative import](https://docs.python.org/3/reference/import.html#package-relative-imports) keeps working when you
rename the enclosing package. This was a reason
[PEP 328](https://peps.python.org/pep-0328/#rationale-for-relative-imports) introduced relative imports.
[PEP 8](https://peps.python.org/pep-0008/#imports) generally prefers absolute imports and the
[Google style guide](https://google.github.io/styleguide/pyguide.html#224-decision) requires them. Enable this rule if
you prefer relative imports within a package.

## Example

```python context file=package/__init__.py

```

```python context file=package/sub/__init__.py

```

```python context file=package/sub/sibling.py

```

```python context file=package/sub/deep.py
thing = 1
```

```python report file=package/sub/module.py
from package.sub import sibling
from package.sub.deep import thing
```

Use instead:

```python context file=package/__init__.py

```

```python context file=package/sub/__init__.py

```

```python context file=package/sub/sibling.py

```

```python context file=package/sub/deep.py
thing = 1
```

```python ok file=package/sub/module.py
from . import sibling
from .deep import thing
```

## Conflicts with Ruff

Ruff's [relative-imports](https://docs.astral.sh/ruff/rules/relative-imports/) bans parent-relative imports by default.
If you set [`ban-relative-imports`](https://docs.astral.sh/ruff/settings/#lint_flake8-tidy-imports_ban-relative-imports)
to `"all"`, Ruff can reverse simplint's fix. When both rules apply to a file, simplint stops before linting. Set
`ban-relative-imports` to `"parents"` or disable one rule.

## What it does

### Active on

- **`from` imports of the file's own package or a package below it**, written by absolute path, such as
  `from package.sub import sibling` in `package/sub/module.py`. A file belongs to a package when its directory holds an
  [`__init__.py`](https://docs.python.org/3/reference/import.html#regular-packages); simplint builds the package name
  from the chain of directories with an `__init__.py` above the file. The fix is safe for a module imported as part of
  its package, where the relative import resolves to the same module.

  ```python context file=package/__init__.py
  # A package.
  ```

  ```python context file=package/sub/__init__.py
  # A subpackage.
  ```

  ```python context file=package/sub/sibling.py
  VALUE = 1
  ```

  ```python report file=package/sub/module.py
  from package.sub import sibling
  ```

  ```python fixed file=package/sub/module.py
  from . import sibling
  ```

### Exceptions

- **Imports from a parent package**: Ruff's [relative-imports](https://docs.astral.sh/ruff/rules/relative-imports/) bans
  relative imports from parent packages by default, so these stay absolute.

  ```python context file=package/__init__.py
  # A package.
  ```

  ```python context file=package/helpers.py
  VALUE = 1
  ```

  ```python context file=package/sub/__init__.py
  # A subpackage.
  ```

  ```python ok file=package/sub/module.py
  from package import helpers
  ```

- **Imports in a directory without an `__init__.py` that Ruff's
  [`namespace-packages`](https://docs.astral.sh/ruff/settings/#namespace-packages) treats as a package**. pytest can
  import a test file there as a top-level module, where relative imports fail.

  ```python context file=namespace/other.py
  VALUE = 1
  ```

  ```python ok file=namespace/test_module.py
  from namespace import other
  ```

- **A top-level `if` or `elif` that tests `__name__` against a string with `==` or `!=`** exempts the whole module.
  Examples include [`if __name__ == "__main__":`](https://docs.python.org/3/library/__main__.html#name-main) for a file
  run as `python path/to/module.py`, or execnet's `if __name__ == "__channelexec__":` for a module sent to another
  interpreter as source. Relative imports can fail in either case.

  ```python context file=package/__init__.py
  # A package.
  ```

  ```python context file=package/sub/__init__.py
  # A subpackage.
  ```

  ```python context file=package/sub/sibling.py
  VALUE = 1
  ```

  ```python ok file=package/sub/script.py
  from package.sub import sibling

  if __name__ == "__main__":
      print(sibling.VALUE)
  ```

- **The body of a top-level function passed by name as the first argument to a `remote_exec(...)` or
  [`inspect.getsource(...)`](https://docs.python.org/3/library/inspect.html#inspect.getsource) call in the same
  module**. The function's source may run outside its package. simplint matches the call by its final name
  (`remote_exec` or `getsource`).

  ```python context file=package/__init__.py
  # A package.
  ```

  ```python context file=package/sub/__init__.py
  # A subpackage.
  ```

  ```python context file=package/sub/sibling.py
  VALUE = 1
  ```

  ```python ok file=package/sub/worker.py
  def remote(channel):
      from package.sub import sibling

      channel.send(sibling.VALUE)


  def start(gateway):
      gateway.remote_exec(remote)
  ```

- **Projects configured for [flake8-lazy](https://github.com/henryiii/flake8-lazy)**. Its standalone runner can rewrite
  `__lazy_modules__` based on import paths. simplint disables this rule for those projects, even with
  `--select absolute-sibling-import`.

  simplint looks for a `[tool.flake8-lazy]` table in the `pyproject.toml` that defines `[project]` or `[tool.simplint]`.
  It also checks the nearest `.pre-commit-config.yaml`, `.pre-commit-config.yml` or `prek.toml` for a hook with the ID
  `flake8-lazy`. The search stops at the repository root or, outside a repository, at the project being linted. An
  unreadable or invalid hook configuration produces a warning. Listing `flake8-lazy` in another hook's
  `additional_dependencies` only installs the plugin and does not disable this rule.

  ```toml config
  [tool.flake8-lazy]
  ```

  ```python context file=package/__init__.py
  # A package.
  ```

  ```python context file=package/sub/__init__.py
  # A subpackage.
  ```

  ```python context file=package/sub/sibling.py
  VALUE = 1
  ```

  ```python ok file=package/sub/module.py
  from package.sub import sibling
  ```

### Undetected cases

Disable the rule for files that run outside their package without a `__name__` guard. Examples include a build script
invoked by path and a module sent as source by another file.

## Other tools

Ruff tracks relative sibling imports in [PR #16772](https://github.com/astral-sh/ruff/pull/16772) and relative import
management in [issue #1014](https://github.com/astral-sh/ruff/issues/1014).
