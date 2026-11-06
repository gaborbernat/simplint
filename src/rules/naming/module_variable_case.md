Flags module-scope variables whose name is not `UPPER_CASE`, such as `limit` or `logger`. A leading underscore stays as
it is: [module-variable-prefix](SMP404.html) checks it.

## Why is this bad?

Upper case tells a reader the name is a module-level value rather than a local, as
[PEP 8](https://peps.python.org/pep-0008/#constants) writes constants. A module that keeps the same case for all its
constants makes them easy to pick out at every use.

## Example

```python report
limit = 3
logger = logging.getLogger(__name__)

__all__ = ["logger"]
```

Use instead:

```python ok
LIMIT = 3
LOGGER = logging.getLogger(__name__)

__all__ = ["LOGGER"]
```

## Configuration

- [`naming.keep-names`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#naming) in
  `[tool.simplint]`: module-scope names the rule leaves as they are, such as `keep-names = ["urlpatterns"]` under
  `[tool.simplint.naming]`

  ```toml config
  [tool.simplint.naming]
  keep-names = ["limit"]
  ```

  ```python ok
  limit = 3
  ```

- The Python version the code targets, taken from the first of these that applies; see
  [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version):

  1. [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version) in `[tool.ruff]`: for
     the files a pattern matches

     ```toml config
     [tool.ruff.per-file-target-version]
     "legacy/*.py" = "py37"
     ```

     ```python report file=legacy/old.py
     limit = 3
     ```

     ```python report file=new.py
     limit = 3
     ```

  1. [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) in `[tool.ruff]`: for every other file

     ```toml config
     [tool.ruff]
     target-version = "py38"
     ```

     ```python report
     limit = 3
     ```

  1. [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python):
     its lowest version, when Ruff sets neither

     ```toml config
     [project]
     name = "demo"
     requires-python = ">=3.8"
     ```

     ```python report
     limit = 3
     ```

## What it does

### Active on

- **Every module-scope variable**: whatever its visibility, its name should be `UPPER_CASE`. A leading underscore stays
  as it is, so `_timeout` becomes `_TIMEOUT`.

  ```python report
  limit = 3
  _timeout = 5
  ```

- **Names another module in the run imports**: only the case changes, and the leading underscore stays, since the
  importer already depends on the name.

  ```python report file=config.py
  limit = 3
  ```

  ```python context file=consumer.py
  from config import limit

  print(limit)
  ```

- **Names bound with `:=`**: a walrus at module scope binds a module variable like any assignment.

  ```python report
  if (total := 3) > 2:
      pass
  ```

simplint reads `__all__` from assignments, `+=`, `.extend([...])` and `.append("name")` with string literals.

### Exceptions

- **An `__all__` built any other way**: its contents depend on how the module runs, so the rule is skipped. This covers:

  - A call, such as `list(...)` or `.remove(...)`:

    ```python ok
    limit = 3
    __all__ = list(["limit"])
    ```

  - A binding inside a nested block, such as an `if`, `try` or function:

    ```python ok
    limit = 3

    if FLAG:
        __all__ = ["limit"]
    ```

- **Dunder names**: Python and its tools give these their meaning by their exact spelling.

  ```python ok
  __version__ = "1.0"
  ```

- **Class-like names**, such as the type alias `Alias = int`: they follow the `CapWords` convention of classes.

  ```python ok
  Alias = int
  ```

- **Type aliases from a `type` statement**: the statement binds a type, not a value, so the name follows the convention
  of classes, whatever its case.

  ```toml config
  [project]
  requires-python = ">=3.12"
  ```

  ```python ok
  type vector = list[float]
  ```

- **Names also bound by an import, `def` or `class`**: this rule skips them. This covers:

  - An import, such as `from json import dumps`: the import keeps the spelling of what it imports.

    ```python ok
    from json import dumps

    dumps = dumps
    ```

  - A `def` or `class`, such as `def limit()`: [unexported-public-name](SMP403.html) applies the underscore convention
    to functions and classes.

    ```python ok
    def limit() -> int:
        return 3


    limit = limit()
    ```

- **Names bound only as `for`, `with`, `except` or `match` targets**: these hold whatever the statement produces, not a
  value the module defines.

  ```python ok
  for item in range(3):
      pass
  ```

- **`TYPE_CHECKING`**: type checkers recognize the guard by that name whatever its value.

  ```python ok
  from typing import TYPE_CHECKING

  if TYPE_CHECKING:
      pass
  ```

The rule also skips names that a tool or framework looks up by their current spelling, since renaming them breaks the
lookup:

- **pytest's module variables**:
  [`pytestmark`](https://docs.pytest.org/en/stable/example/markers.html#marking-whole-classes-or-modules),
  [`pytest_plugins`](https://docs.pytest.org/en/stable/how-to/plugins.html#requiring-loading-plugins-in-a-test-module-or-conftest-file),
  [`collect_ignore` and `collect_ignore_glob`](https://docs.pytest.org/en/stable/example/pythoncollection.html#customizing-test-collection).

  ```python ok file=tests/test_api.py
  import pytest

  pytestmark = pytest.mark.slow
  ```

- **Every name in a `conf.py`**: [Sphinx](https://www.sphinx-doc.org/en/master/usage/configuration.html) reads its
  settings by their lowercase names.

  ```python ok file=conf.py
  project = "demo"
  ```

- **`revision`, `down_revision`, `branch_labels` and `depends_on` in an
  [Alembic](https://alembic.sqlalchemy.org/en/latest/tutorial.html#create-a-migration-script) revision file**: a module
  that binds `revision` and `down_revision` and defines an `upgrade` function. The names place the file in the migration
  graph.

  ```python ok file=migrations/versions/abc123.py
  revision = "abc123"
  down_revision = None
  branch_labels = None
  depends_on = None


  def upgrade() -> None:
      pass
  ```

- **[gunicorn settings](https://gunicorn.org/reference/settings/)**, such as `bind`, `workers` or `timeout`, in a
  gunicorn configuration file: gunicorn reads them by these exact names.

  A configuration file is one whose name contains `gunicorn`, such as `gunicorn.conf.py`, or a module that binds at
  least three settings, one of them `bind`, `workers` or `worker_class`. Any file can be one, so the second test tells
  it from a module that happens to define a `timeout`.

  ```python ok file=gunicorn.conf.py
  bind = "0.0.0.0:8000"
  workers = 4
  ```

- **`version`, `version_tuple` and `commit_id` in a module that also binds `__version__`**: the names of the version
  file [setuptools-scm](https://setuptools-scm.readthedocs.io/latest/usage/#version-files) writes.

  ```python ok
  __version__ = "1.0"
  version = "1.0"
  version_tuple = (1, 0, 0)
  commit_id = None
  ```

- **`apilevel`, `threadsafety` and `paramstyle` in a module that binds all three**: the
  [PEP 249](https://peps.python.org/pep-0249/#globals) globals of a database driver.

  ```python ok
  apilevel = "2.0"
  threadsafety = 1
  paramstyle = "qmark"
  ```

- **PLY grammar names**: in a module that imports [PLY](https://github.com/dabeaz/ply)'s `lex` or `yacc`, including a
  vendored copy such as `pkg._ply.yacc`, the names `tokens`, `literals`, `precedence`, `states`, `start` and the `t_`
  and `p_` rules, which PLY reads off the module by name.

  ```python ok
  from ply import lex

  tokens = ("NUMBER",)
  ```

- **Every name in a PLY table module**, one that binds `_tabversion`: PLY reads its tables back by name.

  ```python ok
  _tabversion = "3.10"
  limit = 3
  ```

- **`app` and `application` bound to a WSGI, ASGI or [Typer](https://typer.tiangolo.com/tutorial/typer-command/)
  application**: servers and the `typer` command load the application by this name.

  The name must be assigned at the top level of the module from one of these calls: `aiohttp.web.Application`,
  `bottle.Bottle`, Django's `get_wsgi_application` and `get_asgi_application`, `falcon.App`, `falcon.API` and
  `falcon.asgi.App`, `fastapi.FastAPI`, `flask.Flask`, `litestar.Litestar`, `quart.Quart`, `sanic.Sanic`,
  `starlette.applications.Starlette`, `typer.Typer`, and Pyramid's `make_wsgi_app()`; the call is resolved through the
  module's imports.

  ```python ok
  from fastapi import FastAPI

  app = FastAPI()
  ```

- **The names listed in `naming.keep-names`**, under Configuration above: for any other framework that reads a module
  variable by name.

### Fix

The fix renames every use in the file, including the `__all__` entry, and keeps the leading underscore of the name.
**Unsafe fix:** always, as importers outside the run keep the old name, and so do string references in unchecked files,
such as [`mocker.patch`](https://pytest-mock.readthedocs.io/en/latest/usage.html) targets. Code that loads a module
under a name no file of the run defines, such as a test that copies a file's text into a module and reads its names off
the copy, keeps the old name too.

```python report
limit = 3
print(limit)
```

```python fixed
LIMIT = 3
print(LIMIT)
```

A name read in an f-string field with a [`=`](https://docs.python.org/3/reference/lexical_analysis.html#f-strings) debug
specifier keeps its printed text: the field prints the old name, then the value in a plain field.

```python report
limit = 3
print(f"{limit=}")
```

```python fixed
LIMIT = 3
print(f"limit={LIMIT!r}")
```

No fix is offered when:

- another module in the run imports the name or reaches it by a dotted string, such as a `mocker.patch` target

  ```python report file=config.py
  limit = 3
  ```

  ```python context file=consumer.py
  from config import limit

  print(limit)
  ```

- a file of the run reads an attribute by that name off a module it loads by a computed name: the result of
  `importlib.import_module`, `__import__` or `sys.modules[...]`, a parameter annotated `ModuleType`, or a pytest fixture
  that returns such a module

  ```python report file=loader.py
  limit = 3
  ```

  ```python context file=reader.py
  import importlib

  value = importlib.import_module("loader").limit
  ```

- the new name is already used as a name in the file

  ```python report
  limit = 3
  LIMIT = 4
  print(limit, LIMIT)
  ```

- the name shadows a builtin, as in `zip = zip`, where the value reads the builtin under the same name

  ```python report
  zip = zip
  ```

- the old name appears in the text of a string of any file in the run, as written or through an escape such as
  `\N{LATIN SMALL LETTER X}`, other than an entry of an `__all__` list assigned with `=` or `+=`

  ```python report
  limit = 3
  NAME = "limit"
  ```

- the old name is read in a t-string, which keeps the source text of its expressions

  ```toml config
  [project]
  requires-python = ">=3.14"
  ```

  ```python report
  limit = 3
  template = t"{limit}"
  print(template.interpolations[0].expression)
  ```

- the old name also names something a text rename cannot tell apart: a parameter, a local variable, a class attribute, a
  keyword argument or an attribute access

  ```python report
  limit = 3


  def scale(limit: int) -> int:
      return limit
  ```

### Related rules

[module-variable-prefix](SMP404.html) checks the leading underscore of the same variables.
[unexported-public-name](SMP403.html) applies the underscore convention to functions and classes.

## Ruff overlap

Ruff's [mixed-case-variable-in-global-scope](https://docs.astral.sh/ruff/rules/mixed-case-variable-in-global-scope/)
flags only `mixedCase` names. This rule also flags `lower_case` names.

## Other tools

- [pylint](https://github.com/pylint-dev/pylint) reports module-level assignments that do not match `const-naming-style`
  as [invalid-name](https://pylint.readthedocs.io/en/latest/user_guide/messages/convention/invalid-name.html). It
  defaults to `UPPER_CASE` and only reports, where this rule renames the uses in the file.
