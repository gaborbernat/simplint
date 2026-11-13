Flags top-level functions and classes that look public, with no leading underscore, but are missing from `__all__`.

## Why is this bad?

`__all__` declares the public API, and the
[language reference](https://docs.python.org/3/reference/simple_stmts.html#the-import-statement) asks it to contain all
of it. A name that looks public but is missing from `__all__` gets conflicting answers: a star import and Sphinx's
[`automodule`](https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html#directive-option-automodule-members)
skip it, while the
[typing spec](https://typing.python.org/en/latest/spec/distributing.html#library-interface-public-and-private-symbols)
does not make it private. [PEP 8](https://peps.python.org/pep-0008/#public-and-internal-interfaces) settles it with a
leading underscore on internal names, even when `__all__` is set, so readers see the answer wherever the name appears.

## Example

```python report
def exported() -> int:
    return helper()


def helper() -> int:
    return 1


__all__ = ["exported"]
```

Use instead:

```python ok
def exported() -> int:
    return _helper()


def _helper() -> int:
    return 1


__all__ = ["exported"]
```

## What it does

### Active on

- **Top-level functions and classes in a module that defines
  [`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package)**, whose names have no leading
  underscore and are missing from `__all__`: the name looks public, yet the module's declared API leaves it out.

  - `def` functions:

    ```python report
    def exported() -> int:
        return helper()


    def helper() -> int:
        return 1


    __all__ = ["exported"]
    ```

  - classes:

    ```python report
    __all__ = ["run"]


    def run() -> None:
        pass


    class Record:
        pass
    ```

simplint reads `__all__` from assignments, `+=`, `.extend([...])` and `.append("name")` with string literals.

### Exceptions

- **Modules without `__all__`**: they declare no API to compare the names against.

  ```python ok
  def helper() -> int:
      return 1
  ```

- **An `__all__` built any other way**: its contents depend on how the module runs, so nothing is reported.

  - a call, such as `list(...)` or `.remove(...)`:

    ```python ok
    def helper() -> int:
        return 1


    __all__ = list(["helper"])
    ```

  - bound inside a nested block, such as an `if`, `try` or function:

    ```python ok
    def helper() -> int:
        return 1


    if __debug__:
        __all__ = ["helper"]
    ```

- **Test modules**: `test_*.py`, `*_test.py`, `conftest.py`, and files under a `tests` or `test` directory. The test
  runner finds test functions and fixtures by their public names.

  ```python ok file=test_helpers.py
  def helper() -> int:
      return 1


  __all__ = []
  ```

- **Decorated functions and classes**: the decorator may register them.

  ```python ok
  from functools import cache


  @cache
  def helper() -> int:
      return 1


  __all__ = []
  ```

- **Names another module in the run imports**: [missing-from-all](SMP401.html) asks you to add these to `__all__`. The
  same holds for the forms below.

  - `from module import name`, and a dotted string such as a `mocker.patch` target:

    ```python ok file=config.py
    __all__ = ["exported"]


    def exported() -> int:
        return helper() + other()


    def helper() -> int:
        return 1


    def other() -> int:
        return 2
    ```

    ```python context file=cli.py
    from unittest import mock

    from config import helper

    print(helper())

    with mock.patch("config.other"):
        pass
    ```

  - names a file reads off a module loaded by a computed name, such as the result of
    [`importlib.import_module`](https://docs.python.org/3/library/importlib.html#importlib.import_module), off a
    parameter annotated `ModuleType`, or off a module whose spec loads from a path no static expression names:

    ```python ok
    import importlib

    plugins = importlib.import_module("plugins")
    HELPER = plugins.helper


    def helper() -> int:
        return 1


    __all__ = []
    ```

  - names read off the result of a string-named import call, such as `pytest.importorskip("plugins")`, directly or
    through a variable it is assigned to:

    ```python ok file=plugins.py
    __all__ = []


    def helper() -> int:
        return 1
    ```

    ```python context file=test_plugins.py
    import pytest

    helper = pytest.importorskip("plugins").helper
    ```

  - names read off a module loaded from a file path with
    [`importlib.util.spec_from_file_location`](https://docs.python.org/3/library/importlib.html#importlib.util.spec_from_file_location)
    and `module_from_spec`, when the path names a project module, as `Path(__file__).parent / "plugins.py"` does:

    ```python ok file=plugins.py
    __all__ = []


    def helper() -> int:
        return 1
    ```

    ```python context file=test_plugins.py
    import importlib.util
    from pathlib import Path

    spec = importlib.util.spec_from_file_location("plugins", Path(__file__).parent / "plugins.py")
    assert spec is not None
    module = importlib.util.module_from_spec(spec)
    helper = module.helper
    ```

- **Classes in the signature of an exported name**: a class that an exported function's annotations, or an exported
  class's bases and fields, refer to is public API, so renaming it would leave the exported signature naming a private
  class. Classes reached this way follow the same rule.

  ```python ok
  class Record:
      pass


  def load() -> Record:
      return Record()


  __all__ = ["load"]
  ```

- **[pytest hooks](https://docs.pytest.org/en/stable/reference/reference.html#hooks) (`pytest_*`),
  [`load_tests`](https://docs.python.org/3/library/unittest.html#load-tests-protocol), and
  [`setup_module` and `teardown_module`](https://docs.pytest.org/en/stable/how-to/xunit_setup.html#module-level-setup-teardown)**:
  the test runners find them by name.

  ```python ok
  def pytest_configure() -> None:
      pass


  def load_tests(loader: object, tests: object, pattern: object) -> object:
      return tests


  def setup_module() -> None:
      pass


  def teardown_module() -> None:
      pass


  __all__ = []
  ```

- **`setup`**: the entry point Sphinx calls in an [extension](https://www.sphinx-doc.org/en/master/extdev/index.html).

  ```python ok
  def setup(app: object) -> None:
      pass


  __all__ = []
  ```

- **Build backend hooks** of the module that `build-backend` names in `[build-system]`: a build frontend calls the PEP
  517 and PEP 660 hooks by name.

  ```toml config
  [build-system]
  build-backend = "build_backend"
  requires = ["setuptools"]
  backend-path = ["."]
  ```

  ```python ok file=build_backend.py
  def build_wheel(wheel_directory: str) -> str:
      return wheel_directory


  __all__ = []
  ```

- **Coverage plugins**: the `coverage_init` coverage calls on each module that `[tool.coverage.run] plugins` lists,
  found in the project's directory or in a `pythonpath` or `extra-paths` directory of pytest or ty.

  ```toml config
  [tool.coverage.run]
  plugins = ["tasks.coverage_pragmas"]
  ```

  ```python ok file=tasks/coverage_pragmas.py
  def coverage_init(reg: object, options: dict[str, str]) -> None:
      pass


  __all__ = []
  ```

- **The `t_` lexer rules and `p_` grammar productions of a module that imports [PLY](https://github.com/dabeaz/ply)'s
  `lex` or `yacc`**: PLY finds them by name.

  ```python ok
  from ply import lex


  def t_NUMBER(t: object) -> object:
      return t


  def p_expression(p: object) -> None:
      pass


  __all__ = []
  ```

### Fix

The fix renames the name to `_name` everywhere in the file. **Unsafe fix:** always, as importers outside the run keep
the old name, and a renamed class changes its `repr` and the name pickle stores. If the name is meant to be public, add
it to `__all__` instead.

In a public module of a library there is no fix: its names are the API users import, so the report asks for the name in
`__all__` or a rename by hand.

```python report
__all__ = ["exported"]


def exported() -> int:
    return helper()


def helper() -> int:
    return 1
```

```python fixed
__all__ = ["exported"]


def exported() -> int:
    return _helper()


def _helper() -> int:
    return 1
```

A name read in an f-string field with a [`=`](https://docs.python.org/3/reference/lexical_analysis.html#f-strings) debug
specifier keeps its printed text: the field prints the old name, then the value in a plain field.

```python report
__all__ = ["exported"]


def exported() -> str:
    return f"{helper()=}"


def helper() -> int:
    return 1
```

```python fixed
__all__ = ["exported"]


def exported() -> str:
    return f"helper()={_helper()!r}"


def _helper() -> int:
    return 1
```

No fix is offered when:

- the old name is read in a t-string, which keeps the source text of its expressions

  ```toml config
  [project]
  requires-python = ">=3.14"
  ```

  ```python report
  __all__ = ["exported"]


  def exported() -> str:
      return t"{helper}".interpolations[0].expression


  def helper() -> int:
      return 1
  ```

- the new name is already used as a name in the file

  ```python report
  __all__ = ["exported"]


  def exported() -> int:
      return helper()


  def helper() -> int:
      return 1


  def _helper() -> int:
      return 2
  ```

  ```python fixed
  __all__ = ["exported"]


  def exported() -> int:
      return helper()


  def helper() -> int:
      return 1


  def _helper() -> int:
      return 2
  ```

- the old name appears in a string of any file in the run, other than an entry of an `__all__` list assigned with `=` or
  `+=`

  ```python report
  __all__ = ["exported"]


  def exported() -> int:
      return 1


  def helper() -> int:
      return 2


  LABEL = "helper"
  ```

  ```python fixed
  __all__ = ["exported"]


  def exported() -> int:
      return 1


  def helper() -> int:
      return 2


  LABEL = "helper"
  ```

- the name is a function or class whose `__name__` or `__qualname__` a file of the run reads, as `helper.__name__` or
  `type(self).__name__` does, which would return the new name

  ```python report
  __all__ = []


  def helper() -> str:
      return helper.__name__
  ```

  ```python report
  __all__ = []


  class Item:
      def label(self) -> str:
          return type(self).__name__
  ```

- a file of the run sets an attribute of the module under a computed name, as
  [`setattr(module, name, value)`](https://docs.python.org/3/builtins/functions.html#setattr) does

- the build compiles Rust beside the package, with [maturin](https://www.maturin.rs/) or
  [setuptools-rust](https://setuptools-rust.readthedocs.io/), whose code can import a name by its spelling, as PyO3
  does, where no Python file shows it

- the old name also names something a text rename cannot tell apart: a parameter, a local variable, a class attribute, a
  keyword argument or an attribute access

  ```python report
  __all__ = ["exported"]


  def exported(helper: int) -> int:
      return helper


  def helper() -> int:
      return 1
  ```

  ```python fixed
  __all__ = ["exported"]


  def exported(helper: int) -> int:
      return helper


  def helper() -> int:
      return 1
  ```

### Related rules

[module-variable-prefix](SMP404.html) applies the same convention to module variables.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issue [#12962](https://github.com/astral-sh/ruff/issues/12962)
  (item 1). Issue [#18380](https://github.com/astral-sh/ruff/issues/18380) asks instead for a leading underscore on
  names no other module imports.
