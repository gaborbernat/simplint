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

  - the declarations of the module's `.pyi` stub

    ```python ok file=config.py
    LIMIT = 3
    __all__ = ["LIMIT"]
    ```

    ```python context file=config.pyi
    LIMIT: int
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
written on one line.

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
