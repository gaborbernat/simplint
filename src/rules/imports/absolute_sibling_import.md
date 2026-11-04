Flags `from ... import` statements that name the file's own package, or a package below it, by its absolute path.

## Why is this bad?

An absolute import repeats the package name, so renaming or vendoring the package means editing every such import. A
[relative import](https://docs.python.org/3/reference/import.html#package-relative-imports)
([PEP 328](https://peps.python.org/pep-0328/)) shows at a glance that the target lives next to the importing module.

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
turn one of the two rules off.

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

### Undetected cases

A build script that a tool runs by path without a `__name__` check, or a module that only another file sends as source,
is not detected; turn the rule off for that file.
