Flags imports of selected backports when the file targets a Python version with a standard library alternative.

## Why is this bad?

If the project only needs an API that its minimum Python version provides, the standard library import saves users an
extra dependency. For example, Python 3.11 added [`tomllib`](https://docs.python.org/3/library/tomllib.html), and
[Tomli's compatibility example](https://github.com/hukkin/tomli#building-a-tomlitomllib-compatibility-layer) requires
`tomli` only below 3.11. Check which features you use before dropping a backport that offers a newer API.

## Example

```toml config
[project]
name = "demo"
requires-python = ">=3.11"
```

```python report
from tomli import loads

print(loads("name = 'demo'"))
```

```python fixed
from tomllib import loads

print(loads("name = 'demo'"))
```

## Configuration

- The [target Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version) for
  the file

  ```toml config
  [tool.ruff]
  target-version = "py310"
  ```

  ```python report
  from importlib_metadata import version

  print(version("simplint"))
  ```

## What it does

### Active on

- **`tomli`, `exceptiongroup`, `async_timeout` and `taskgroup`** from Python 3.11, and **`importlib_metadata` and
  `importlib_resources`** from Python 3.10, when the target version is at least the version listed. For `async_timeout`,
  `taskgroup`, `importlib_metadata` and `importlib_resources`, the rule reports the import for manual review because the
  APIs may differ.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.11"
  ```

  ```python report
  from taskgroup import TaskGroup

  VALUE = TaskGroup
  ```

- **`exceptiongroup` imports of [`ExceptionGroup`](https://docs.python.org/3/builtins/exceptions.html#ExceptionGroup)
  and [`BaseExceptionGroup`](https://docs.python.org/3/builtins/exceptions.html#BaseExceptionGroup)**, which the
  builtins define from Python 3.11 ([PEP 654](https://peps.python.org/pep-0654/)).

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.11"
  ```

  ```python report
  # Demo module.
  from exceptiongroup import ExceptionGroup

  VALUE = ExceptionGroup("failed", [ValueError()])
  ```

  ```python fixed
  # Demo module.

  VALUE = ExceptionGroup("failed", [ValueError()])
  ```

### Exceptions

- **`import tomli` without an alias**: the module binds a different name, so every use of `tomli` would need renaming.
  The rule reports the import without a fix.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.11"
  ```

  ```python report
  import tomli

  print(tomli.loads("name = 'demo'"))
  ```

- **Imports directly in a `try` body, handler, `else` or `finally` clause**. These may implement a fallback.
  [dependency-import-guard](SMP303.html) checks guards around required imports.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.11"
  ```

  ```python ok
  try:
      from tomli import loads
  except ImportError:
      from tomllib import loads
  ```

- **`exceptiongroup` imports that name anything else**, such as `from exceptiongroup import catch`: the other names
  still come from the backport.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.11"
  ```

  ```python ok
  from exceptiongroup import catch

  VALUE = catch
  ```

- **`exceptiongroup` names another module of the run imports from this one, or `__all__` lists**: deleting the import
  would break that module, so it has no fix. Ruff's [A004](https://docs.astral.sh/ruff/rules/builtin-import-shadowing/)
  reports the import instead, and this rule reports nothing for it.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.11"
  ```

  ```python context file=pkg/consumer.py
  from pkg.compat import ExceptionGroup

  VALUE = ExceptionGroup("failed", [ValueError()])
  ```

  ```python ok file=pkg/compat.py
  from exceptiongroup import ExceptionGroup

  __all__ = ["ExceptionGroup"]
  ```

### Fix

- **`tomli`**: the module name changes to `tomllib` in `from tomli import loads` and `import tomli as toml`, keeping the
  imported names and the alias. The fix is safe.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.11"
  ```

  ```python report
  import tomli as toml

  print(toml.loads("name = 'demo'"))
  ```

  ```python fixed
  import tomllib as toml

  print(toml.loads("name = 'demo'"))
  ```

- **`exceptiongroup`**: an import of only `ExceptionGroup` and `BaseExceptionGroup`, without aliases and alone on its
  line, is deleted. The fix is safe because the builtins define both names. When the statement is the only one in its
  block, the fix replaces it with `pass`. Names another module imports from this one, or `__all__` lists, keep the
  import and get no fix.

  ```toml config
  [project]
  name = "demo"
  requires-python = ">=3.11"
  ```

  ```python report
  # Demo module.
  from exceptiongroup import BaseExceptionGroup

  VALUE = BaseExceptionGroup
  ```

  ```python fixed
  # Demo module.

  VALUE = BaseExceptionGroup
  ```

The fix leaves `pyproject.toml` alone. Remove the dependency after checking whether any supported code still needs it.

## Ruff overlap

Ruff's [builtin-import-shadowing](https://docs.astral.sh/ruff/rules/builtin-import-shadowing/) (A004) reports an
unaliased `exceptiongroup` import of `ExceptionGroup` or `BaseExceptionGroup` that has no fix here: one that another
module takes from this one, one that `__all__` lists, or one that shares its line with other code. This rule leaves
those reports to A004, whether or not the project selects it, and keeps the imports with a fix, which A004 does not
remove. Names the `flake8-builtins` ignorelist allows are not A004's reports, so this rule keeps them too, and so does
an import that also names something A004 does not report, such as an alias.

## Other tools

Ruff's [UP026](https://docs.astral.sh/ruff/rules/deprecated-mock-import/) replaces imports from `mock`.
[UP035](https://docs.astral.sh/ruff/rules/deprecated-import/) checks deprecated imports, including selected names from
`typing_extensions`.
