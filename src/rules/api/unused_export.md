Flags names in `__all__` that no other module of the run imports. A name nothing imports is not part of any interface,
so listing it makes the module look larger than it is.

## Why is this bad?

`__all__` tells readers, and tools such as type checkers and documentation generators, which names a module offers. A
name nothing imports is not part of any interface; listing it makes the module look larger than it is, and keeps a stale
entry after its last user is gone.

## Example

```python context file=cli.py
from parser import parse

parse("")
```

```python report file=parser.py
def parse(text: str) -> list[str]:
    return tokenize(text)


def tokenize(text: str) -> list[str]:
    return text.split()


__all__ = ["parse", "tokenize"]  # only `parse` is imported elsewhere
```

Use instead:

```python context file=cli.py
from parser import parse

parse("")
```

```python ok file=parser.py
def parse(text: str) -> list[str]:
    return _tokenize(text)


def _tokenize(text: str) -> list[str]:
    return text.split()


__all__ = ["parse"]
```

## Configuration

- [`project-kind`](https://simplint.readthedocs.io/en/latest/configuration/project.html#library-or-application) in
  `[tool.simplint]`: whether the project is a library or an application

  ```toml config
  [build-system]
  requires = ["setuptools"]
  build-backend = "setuptools.build_meta"

  [project]
  name = "demo"

  [tool.simplint]
  project-kind = "application"
  ```

  ```python report file=demo/core.py
  def run() -> int:
      return 1


  __all__ = ["run"]
  ```

## What it does

### Active on

- **Names in [`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package) that no other module
  of the run imports**. Whether the fix is safe depends on who else may import the name:

  - in an application, the fix is safe: nothing outside the run imports it

    ```python report file=config.py
    LIMIT = 3


    def local() -> int:
        return 1


    __all__ = ["LIMIT", "local"]
    ```

    ```python context file=cli.py
    from config import LIMIT

    print(LIMIT)
    ```

  - in a library (`project-kind` is `library`), the fix is **unsafe**: code outside the run may import it. The `__all__`
    of a public module is the interface users import, so the rule leaves it alone; it checks the private modules of the
    library

    ```toml config
    [build-system]
    requires = ["setuptools"]
    build-backend = "setuptools.build_meta"

    [project]
    name = "demo"
    ```

    ```python report file=demo/_core.py
    def run() -> int:
        return 1


    __all__ = ["run"]
    ```

    ```python ok file=demo/core.py
    def run() -> int:
        return 1


    __all__ = ["run"]
    ```

  These count as imports, as for [missing-from-all](SMP401.html):

  - `from module import name`

    ```python ok file=config.py
    LIMIT = 3
    __all__ = ["LIMIT"]
    ```

    ```python context file=cli.py
    from config import LIMIT

    print(LIMIT)
    ```

  - `module.name` after `import module`

    ```python ok file=config.py
    LIMIT = 3
    __all__ = ["LIMIT"]
    ```

    ```python context file=cli.py
    import config

    print(config.LIMIT)
    ```

  - `module.name` read off a module loaded from a file path with `importlib.util.spec_from_file_location` and
    `module_from_spec`, when the path names a project module

    ```python ok file=config.py
    LIMIT = 3
    __all__ = ["LIMIT"]
    ```

    ```python context file=cli.py
    import importlib.util
    from pathlib import Path

    spec = importlib.util.spec_from_file_location("config", Path(__file__).parent / "config.py")
    assert spec is not None
    module = importlib.util.module_from_spec(spec)
    print(module.LIMIT)
    ```

  - a string key read off the globals `runpy.run_module` returns, or `runpy.run_path` of a path that names a project
    module

    ```python ok file=config.py
    LIMIT = 3
    __all__ = ["LIMIT"]
    ```

    ```python context file=cli.py
    import runpy

    print(runpy.run_module("config")["LIMIT"])
    ```

  - a dotted path a test patches, as `mock.patch("config.LIMIT")`

    ```python ok file=config.py
    LIMIT = 3
    __all__ = ["LIMIT"]
    ```

    ```python context file=test_cli.py
    from unittest import mock


    def test_limit() -> None:
        with mock.patch("config.LIMIT", 5):
            pass
    ```

  - the entry points `pyproject.toml` declares

    ```toml config
    [project]
    name = "demo"

    [project.scripts]
    run = "config:main"
    ```

    ```python ok file=config.py
    def main() -> None:
        pass


    __all__ = ["main"]
    ```

  - `module:attribute` strings passed to `pkgutil.resolve_name` or stored as an argparse `handler` with `set_defaults`

  - setuptools dynamic `attr` values and command classes, mypy and Pylint plugins, pytest plugins loaded by `addopts`,
    and legacy Poetry plugin entry points declared in `pyproject.toml`

  - local PDM build hooks in `pdm_build.py` or the file named by `[tool.pdm.build] custom-hook`

  - the coverage plugin `pyproject.toml` lists under `[tool.coverage.run] plugins`, whose `coverage_init` coverage
    calls; the module is found in the project's directory or in a `pythonpath` or `extra-paths` directory of pytest or
    ty

    ```toml config
    [tool.coverage.run]
    plugins = ["tasks.coverage_pragmas"]
    ```

    ```python ok file=tasks/coverage_pragmas.py
    def coverage_init(reg: object, options: dict[str, str]) -> None:
        pass


    __all__ = ["coverage_init"]
    ```

  - the declarations of the module's `.pyi` stub

    ```python ok file=config.py
    LIMIT = 3
    __all__ = ["LIMIT"]
    ```

    ```python context file=config.pyi
    LIMIT: int
    ```

- **Reads of `__all__` off a module the run cannot name**: `sys.modules["json"].__all__` reads the standard library's
  list, not one of the run's, so it exempts none of them.

  ```python report file=config.py
  LIMIT = 3


  def local() -> int:
      return 1


  __all__ = ["LIMIT", "local"]
  ```

  ```python context file=cli.py
  from config import LIMIT

  print(LIMIT)
  ```

  ```python context file=tests/test_json.py
  import sys


  def test_json_lists_names() -> None:
      module = sys.modules["json"]
      assert "dumps" in module.__all__
  ```

- **Modules only tests load as a value**: a test that passes a module on, as `monkeypatch.setattr(config, ...)` does,
  may set its names but reads none through it, so a listed name that no production code reads is still reported.

  ```python report file=config.py
  LIMIT = 3


  def local() -> int:
      return 1


  __all__ = ["LIMIT", "local"]
  ```

  ```python context file=cli.py
  from config import LIMIT

  print(LIMIT)
  ```

  ```python context file=tests/test_config.py
  import config


  def test_patch(monkeypatch) -> None:
      monkeypatch.setattr(config, "LIMIT", 1)
  ```

### Exceptions

- **Public modules of a library**: their `__all__` declares the interface users outside the project import, so no name
  in it needs an importer in the run.

  ```toml config
  [build-system]
  requires = ["setuptools"]
  build-backend = "setuptools.build_meta"

  [project]
  name = "demo"
  ```

  ```python ok file=demo/core.py
  def run() -> int:
      return 1


  __all__ = ["run"]
  ```

- **Modules another file star-imports**: `import *` hands every listed name to the importing module, which may use it
  there.

  ```python ok file=config.py
  LIMIT = 3


  def local() -> int:
      return 1


  __all__ = ["LIMIT", "local"]
  ```

  ```python context file=cli.py
  from config import *

  print(LIMIT, local())
  ```

- **Modules whose `__all__` another file reads**: an import of `__all__`, or `module.__all__` read off an import of the
  module, uses every name the list holds, as an `import *` does.

  ```python ok file=config.py
  LIMIT = 3
  local = 1


  __all__ = ["LIMIT", "local"]
  ```

  ```python context file=cli.py
  import config

  print([getattr(config, name) for name in config.__all__])
  ```

- **Package `__init__.py` files**: they list what the package re-exports, and Ruff, which counts their imports as
  re-exports, puts any name left out back into `__all__`.

  ```python ok file=pkg/__init__.py
  from pkg.core import run

  __all__ = ["run"]
  ```

  ```python context file=pkg/core.py
  def run() -> int:
      return 1
  ```

- **Modules production code loads as a value**: a module that a file of the run binds and passes on, as
  `CONFIG = settings` does, may have any name read through that value, so none of its names is reported.

  ```python ok file=config.py
  LIMIT = 3


  def local() -> int:
      return 1


  __all__ = ["LIMIT", "local"]
  ```

  ```python context file=cli.py
  import config as settings

  CONFIG = settings
  ```

- **Submodules a star import binds through their package's `__all__`**: `from pkg import *` in production code binds
  each submodule the package's `__all__` lists, so the names of that submodule may be read under its own name. A star
  import in a test binds nothing that production reads, so it exempts none of them.

  ```python ok file=pkg/sub.py
  LIMIT = 3


  def func() -> int:
      return LIMIT


  __all__ = ["LIMIT", "func"]
  ```

  ```python context file=pkg/__init__.py
  __all__ = ["sub"]
  ```

  ```python context file=cli.py
  from pkg import *

  print(sub.func(), sub.LIMIT)
  ```

- **Modules with a module-level `__getattr__`**: it serves names `__all__` lists without the module binding them, as a
  deprecation shim or a lazy loader does, so whether anything imports them shows only outside the run.

  ```python ok file=config.py
  __all__ = ["LIMIT"]


  def __getattr__(name: str) -> int:
      return 1
  ```

- **Types in the signature of another exported name**: a class or alias that an exported function's annotations, or an
  exported class's bases and fields, refer to is public API, so `__all__` may list it without any file importing it.
  Types reached this way follow the same rule.

  ```python ok file=parser.py
  Tokens = list[str]


  def parse(text: str) -> Tokens:
      return text.split()


  __all__ = ["Tokens", "parse"]
  ```

  ```python context file=cli.py
  from parser import parse

  parse("")
  ```

- **Test modules**, as defined in [test-class](SMP801.html): these are not checked.

  ```python ok file=test_config.py
  __all__ = ["check"]


  def check() -> None:
      pass
  ```

- **Modules that build `__all__` dynamically**, such as with `list(...)`: the listed names are not known.

  ```python ok
  LIMIT = 3
  __all__ = list(["LIMIT"])
  ```

### Fix

The fix removes the names from `__all__`: their lines when each name has a line of its own, or the names from a list
written on one line. When several names are unused, only the first report carries the fix, since it removes all of them
at once; the others have none.

```python report file=config.py
LIMIT = 3


def local() -> int:
    return 1


__all__ = [
    "LIMIT",
    "local",
]
```

```python fixed file=config.py
LIMIT = 3


def local() -> int:
    return 1


__all__ = [
    "LIMIT",
]
```

```python context file=cli.py
from config import LIMIT

print(LIMIT)
```

A name in parentheses goes with its parentheses:

```python context file=cli.py
from config import ZETA, GAMMA

print(ZETA, GAMMA)
```

```python report file=config.py
ZETA = 1
BETA = 2
GAMMA = 3

__all__ = "ZETA", ("BETA"), "GAMMA"
```

```python fixed file=config.py
ZETA = 1
BETA = 2
GAMMA = 3

__all__ = "ZETA", "GAMMA"
```

```python report
def parse() -> int:
    return 1


def tokenize() -> int:
    return 2


__all__ = ["parse", "tokenize"]
```

```python fixed
def parse() -> int:
    return 1


def tokenize() -> int:
    return 2


__all__ = []
```

No fix is offered when:

- the list holds a comment and its names share lines, since the comment may belong to a removed name

  ```python report
  def parse() -> int:
      return 1


  def tokenize() -> int:
      return 2


  # fmt: off
  __all__ = [
      "parse", "tokenize",  # the entry points
  ]
  # fmt: on
  ```

  ```python fixed
  def parse() -> int:
      return 1


  def tokenize() -> int:
      return 2


  # fmt: off
  __all__ = [
      "parse", "tokenize",  # the entry points
  ]
  # fmt: on
  ```

- more than one statement builds `__all__`, such as `__all__ += [...]`, since there is no one place to remove a name
  from

  ```python report
  def parse() -> int:
      return 1


  def tokenize() -> int:
      return 2


  __all__ = ["parse"]
  __all__ += ["tokenize"]
  ```

  ```python fixed
  def parse() -> int:
      return 1


  def tokenize() -> int:
      return 2


  __all__ = ["parse"]
  __all__ += ["tokenize"]
  ```

### Related rules

[missing-from-all](SMP401.html) is the counterpart, adding the names other modules import: together they keep `__all__`
to the names the rest of the project uses. Once a name leaves `__all__`, [unexported-public-name](SMP403.html) asks for
a leading underscore, so what looks public is what other modules use.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issue [#12962](https://github.com/astral-sh/ruff/issues/12962)
  (item 2: `__all__` entries nothing uses).
