Flags `from ... import` statements that name the file's own package, or a package below it, by its absolute path.

## Why is this bad?

An absolute import repeats the package name, so renaming, moving or vendoring the package means editing every such
import. Letting a package be rearranged without editing its sub-packages was the main reason
[PEP 328](https://peps.python.org/pep-0328/#rationale-for-relative-imports) added relative imports, and a
[relative import](https://docs.python.org/3/reference/import.html#package-relative-imports) shows at a glance that the
target lives next to the importing module. [PEP 8](https://peps.python.org/pep-0008/#imports) prefers absolute imports
and the [Google style guide](https://google.github.io/styleguide/pyguide.html#224-decision) forbids relative ones. This
rule follows PEP 328 and flags absolute in-package imports.

## Example

```python context file=package/__init__.py

```

```python context file=package/sub/__init__.py

```

```python context file=package/sub/sibling.py

```

```python context file=package/sub/deep.py

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

```

```python ok file=package/sub/module.py
from . import sibling
from .deep import thing
```

## Conflicts with Ruff

Ruff's [relative-imports](https://docs.astral.sh/ruff/rules/relative-imports/) bans only relative imports from parent
packages by default, which fits this rule.

Ruff's [relative-imports](https://docs.astral.sh/ruff/rules/relative-imports/) with
[`ban-relative-imports`](https://docs.astral.sh/ruff/settings/#lint_flake8-tidy-imports_ban-relative-imports) set to
`"all"` contradicts it: it asks for the absolute import this rule replaces, and its fix turns the relative import back.
simplint stops with an error before linting while both apply to a file; set `ban-relative-imports` to `"parents"`, or
turn one of the two rules off. A project that runs flake8-lazy has no such conflict, since this rule stays off there.

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
  [`namespace-packages`](https://docs.astral.sh/ruff/settings/#namespace-packages) makes a package**: pytest imports a
  test module there as a top-level module, where a relative import fails.

  ```python context file=namespace/other.py
  VALUE = 1
  ```

  ```python ok file=namespace/test_module.py
  from namespace import other
  ```

- **A module with a top-level `if` that compares `__name__` with a string**, such as
  [`if __name__ == "__main__":`](https://docs.python.org/3/library/__main__.html#name-main) for a file run as
  `python path/to/module.py`, or execnet's `if __name__ == "__channelexec__":` for a module sent to another interpreter
  as source: the code runs outside its package, where a relative import fails.

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
  module**: that call turns the function into source text that runs elsewhere, outside the package.

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


  execnet.remote_exec(remote)
  ```

- **Every file of a project that runs [flake8-lazy](https://github.com/henryiii/flake8-lazy)**: flake8-lazy rewrites
  each module's `__lazy_modules__` list from its absolute imports. In pipx, a relative import made it write a list that
  `ty` rejects (99 errors), and flake8-lazy and this fix then keep rewriting each other's output, so the rule stays off.
  An explicit `--select absolute-sibling-import` does not turn it back on for such a project.

  The project runs flake8-lazy when one of its `pyproject.toml` files, the one with `[project]` or the one with
  `[tool.simplint]`, has a `[tool.flake8-lazy]` table, or when the nearest `.pre-commit-config.yaml`,
  `.pre-commit-config.yml` or `prek.toml` from the project upwards, stopping at the repository root, or at the linted
  project outside a repository, has a hook with id `flake8-lazy`. A hook configuration simplint cannot read or parse
  counts as none, with a warning. A `flake8-lazy` entry in another hook's `additional_dependencies` does not count,
  since it only installs the plugin into flake8.

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

A build script that a tool runs by path without a `__name__` check, or a module that only another file sends as source,
is not detected; turn the rule off for that file.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rule
  [TID252](https://docs.astral.sh/ruff/rules/relative-imports/), which bans relative imports, the opposite direction;
  open pull request [#16772](https://github.com/astral-sh/ruff/pull/16772); open issue
  [#1014](https://github.com/astral-sh/ruff/issues/1014).
