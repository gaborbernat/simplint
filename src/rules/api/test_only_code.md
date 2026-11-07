Flags module-level functions and classes that only test modules use, since they belong in the tests.

## Why is this bad?

Code that only tests call is test support, and it ships with the product. Readers take it for part of the package, and
it outlives the tests that needed it. Moving it into the tests, or into a test helper module, keeps the package to what
the program runs.

## Example

```python context file=tests/test_builders.py
from app.builders import build_fixture


def test_build() -> None:
    assert build_fixture() == {"id": 1}
```

```python report file=app/builders.py
def build_fixture() -> dict[str, int]:  # only tests call it
    return {"id": 1}
```

Use instead:

```python context file=tests/test_builders.py
from app.builders import build_fixture


def test_build() -> None:
    assert build_fixture() == {"id": 1}
```

```python ok file=tests/helpers.py
def build_fixture() -> dict[str, int]:
    return {"id": 1}
```

## What it does

### Active on

- **Module-level functions and classes** whose every reference in the run comes from a test module. A reference is an
  import, a call, an attribute read through the module, or a dotted string the index records. Methods are not covered
  yet, so a method used only by tests is not reported.

  ```python report file=app/builders.py
  def build_fixture() -> int:
      return 1
  ```

  ```python context file=tests/test_builders.py
  from app.builders import build_fixture


  def test_build() -> None:
      assert build_fixture() == 1
  ```

- **References through the module or a dotted string**: `module.name` in a test and a `"pkg.module.name"` string, such
  as the target of `mock.patch`, count as references to `name`.

  ```python report file=app/builders.py
  def build_fixture() -> int:
      return 1
  ```

  ```python context file=tests/test_builders.py
  from unittest import mock

  import app.builders as builders


  def test_build() -> None:
      assert builders.build_fixture() == 1
      with mock.patch("app.builders.build_fixture"):
          pass
  ```

- **Classes**: a class with no reference outside the tests is reported under the same condition.

  ```python report file=app/records.py
  class Record:
      pass
  ```

  ```python context file=tests/test_records.py
  from app.records import Record


  def test_record() -> None:
      assert Record()
  ```

### Exceptions

- **Names the run uses outside the tests**: a single use from application code keeps the name.

  ```python ok file=app/shared.py
  def shared() -> int:
      return 1
  ```

  ```python context file=app/service.py
  from app.shared import shared


  def run() -> int:
      return shared()
  ```

  ```python context file=tests/test_shared.py
  from app.shared import shared


  def test_shared() -> None:
      assert shared() == 1
  ```

- **A library's public API**: a name a public module offers, or that a user of the library may import, is public even
  when the tests are its only importers. The docstring rules decide what is public, so this rule reuses that decision.

  ```toml config
  [build-system]
  requires = ["setuptools"]

  [project]
  name = "mylib"
  ```

  ```python ok file=mylib/api.py
  def build_fixture() -> int:
      return 1
  ```

  ```python context file=tests/test_api.py
  from mylib.api import build_fixture


  def test_build() -> None:
      assert build_fixture() == 1
  ```

- **Names with no reference at all**: dead code is a different concern, so a function nothing calls is not reported
  here.

  ```python ok
  def unused() -> int:
      return 1
  ```

- **Entry points** named in `pyproject.toml`, which the program runs without any import.

  ```toml config
  [project]
  name = "tool"

  [project.scripts]
  tool = "app.cli:main"
  ```

  ```python ok file=app/cli.py
  def main() -> None:
      pass
  ```

  ```python context file=tests/test_cli.py
  from app.cli import main


  def test_main() -> None:
      main()
  ```

- **Modules a `.pth` file loads at startup**: the interpreter runs the `import` lines of a `.pth` file in a source
  directory when it starts, so a name those lines read is kept. A path line runs nothing.

  ```python ok file=app/hook.py
  def _run() -> None:
      print("hook")
  ```

  ```python context file=app.pth
  # fmt: off
  import app.hook; app.hook._run()
  # fmt: on
  ```

  ```python context file=tests/test_hook.py
  from app.hook import _run


  def test_run() -> None:
      _run()
  ```

- **Decorated definitions**: a decorator may register the definition with something outside the run, such as a plugin
  list or a framework, so any decorated function or class is kept.

  ```python ok file=app/registry.py
  from functools import cache


  @cache
  def build_fixture() -> int:
      return 1
  ```

  ```python context file=tests/test_registry.py
  from app.registry import build_fixture


  def test_build() -> None:
      assert build_fixture() == 1
  ```

- **Dunder names** such as `__getattr__`, which the interpreter reads without an import.

  ```python ok file=app/lazy.py
  def __getattr__(name: str) -> int:
      return 1
  ```

  ```python context file=tests/test_lazy.py
  from app.lazy import __getattr__


  def test_lazy() -> None:
      assert __getattr__("x") == 1
  ```

- **Names the module uses itself**: a call from elsewhere in the module, or a name listed in `__all__`, keeps the
  definition, since the module needs it whatever the tests do. A recursive call inside the definition does not count.

  ```python ok file=app/tools.py
  def normalize(text: str) -> str:
      return text.strip()


  def clean(text: str) -> str:
      return normalize(text)
  ```

  ```python context file=tests/test_tools.py
  from app.tools import normalize


  def test_normalize() -> None:
      assert normalize(" a ") == "a"
  ```

- **Test modules**: a module that is itself a test module is not checked.

  ```python ok file=tests/helpers.py
  def make_user() -> str:
      return "alice"
  ```

The rule has no fix, because moving code into the tests changes the layout of the project, which the author decides.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
