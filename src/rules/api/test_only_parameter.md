Flags a parameter with a default that only test calls pass, while production calls leave it at the default.

## Why is this bad?

A parameter that only tests set is a seam for the test suite in production code. Callers see an option the program never
uses, and the variation it offers lives where no reader expects it. Keeping the variation in the tests, through a helper
or a fixture, leaves the production signature to what the program does.

## Example

```python context file=app/service.py
from app.mail import send


def notify(address: str) -> None:
    send(address)
```

```python report file=app/mail.py
def send(address: str, urgent: bool = False) -> None:
    print(address, urgent)
```

```python context file=tests/test_mail.py
from app.mail import send


def test_send() -> None:
    send("a@b.c", urgent=True)
```

Use instead:

```python ok file=app/mail.py
def send(address: str) -> None:
    print(address, False)
```

```python context file=tests/test_mail.py
from app.mail import send


def test_send() -> None:
    send("a@b.c")
```

## What it does

For each module-level function, the rule compares the calls that production code makes with the calls tests make. A
production call that passes an argument by position or by keyword counts for that parameter, and so does one that
unpacks `**kwargs`, which may pass any keyword.

### Active on

- **Defaulted parameters that no production call passes**, when at least one test call passes them and production code
  calls the function at least once. A required parameter is always passed, so it is never reported.

  ```python context file=app/service.py
  from app.mail import send


  def notify(address: str) -> None:
      send(address)
  ```

  ```python report file=app/mail.py
  def send(address: str, urgent: bool = False) -> None:
      print(address, urgent)
  ```

  ```python context file=tests/test_mail.py
  from app.mail import send


  def test_send() -> None:
      send("a@b.c", urgent=True)
  ```

- **Keyword-only parameters**, which a call passes only by keyword.

  ```python context file=app/service.py
  from app.mail import send


  def notify(address: str) -> None:
      send(address)
  ```

  ```python report file=app/mail.py
  def send(address: str, *, urgent: bool = False) -> None:
      print(address, urgent)
  ```

  ```python context file=tests/test_mail.py
  from app.mail import send


  def test_send() -> None:
      send("a@b.c", urgent=True)
  ```

### Exceptions

- **Parameters production code passes**: a single production call that passes the parameter keeps it.

  ```python context file=app/service.py
  from app.mail import send


  def notify(address: str) -> None:
      send(address, urgent=True)
  ```

  ```python ok file=app/mail.py
  def send(address: str, urgent: bool = False) -> None:
      print(address, urgent)
  ```

  ```python context file=tests/test_mail.py
  from app.mail import send


  def test_send() -> None:
      send("a@b.c", urgent=True)
  ```

- **Modules a program runs**: a module with an `if __name__ == "__main__":` block runs as production code even when a
  test imports it, so its calls count as production calls.

  ```python context file=app/notify.py
  from app.mail import send


  def notify(address: str) -> None:
      send(address)
  ```

  ```python ok file=app/mail.py
  def send(address: str, urgent: bool = False) -> None:
      print(address, urgent)
  ```

  ```python context file=app/service.py
  from app.mail import send


  def escalate(address: str) -> None:
      send(address, urgent=True)


  if __name__ == "__main__":
      escalate("a@b.c")
  ```

  ```python context file=tests/test_service.py
  from app.service import escalate


  def test_escalate() -> None:
      escalate("a@b.c")
  ```

  A module that production code star-imports runs as production code too, so its calls count the same way:

  ```python context file=app/notify.py
  from app.mail import send


  def notify(address: str) -> None:
      send(address)
  ```

  ```python ok file=app/mail.py
  def send(address: str, urgent: bool = False) -> None:
      print(address, urgent)
  ```

  ```python context file=app/service.py
  from app.mail import send


  def escalate(address: str) -> None:
      send(address, urgent=True)
  ```

  ```python context file=app/__main__.py
  from app.service import *


  escalate("a@b.c")
  ```

  ```python context file=tests/test_service.py
  from app.service import escalate


  def test_escalate() -> None:
      escalate("a@b.c")
  ```

- **Functions with no production call**: a function only tests call is test support, which `test-only-code` reports
  instead.

  ```python ok file=app/mail.py
  def send(address: str, urgent: bool = False) -> None:
      print(address, urgent)
  ```

  ```python context file=tests/test_mail.py
  from app.mail import send


  def test_send() -> None:
      send("a@b.c", urgent=True)
  ```

- **Functions used other than as a call**: passed as a value, or named by a dotted string or an entry point, where the
  calls that reach them are not all visible.

  ```python ok file=app/mail.py
  def send(address: str, urgent: bool = False) -> None:
      print(address, urgent)


  HANDLERS = [send]
  ```

  ```python context file=tests/test_mail.py
  from app.mail import send


  def test_send() -> None:
      send("a@b.c", urgent=True)
  ```

- **Decorated functions**: a decorator may register or wrap the function, so its calls may not be the ones written.

  ```python ok
  from functools import cache


  @cache
  def send(address: str, urgent: bool = False) -> None:
      print(address, urgent)


  send("a@b.c")
  ```

- **Public functions of a library**: code outside the project may call them without the argument.

  ```toml config
  [build-system]
  requires = ["setuptools"]

  [project]
  name = "mylib"
  ```

  ```python ok file=mylib/api.py
  def send(address: str, urgent: bool = False) -> None:
      print(address, urgent)
  ```

  ```python context file=tests/test_api.py
  from mylib.api import send


  def test_send() -> None:
      send("a@b.c", urgent=True)
  ```

- **A program's entry**: a call from an `if __name__ == "__main__":` block or a `__main__.py` runs the module as a
  program, whose arguments come from the command line, so `main(argv=None)` leaves `argv` out on purpose.

  ```python ok file=app/cli.py
  def main(argv: list[str] | None = None) -> int:
      return len(argv or [])


  if __name__ == "__main__":
      main()
  ```

  ```python context file=tests/test_cli.py
  from app.cli import main


  def test_main() -> None:
      assert main(["a"]) == 1
  ```

  ```python ok file=app/run.py
  def main(argv: list[str] | None = None) -> int:
      return len(argv or [])
  ```

  ```python context file=app/__main__.py
  from app.run import main

  main()
  ```

  ```python context file=tests/test_run.py
  from app.run import main


  def test_main() -> None:
      assert main(["a"]) == 1
  ```

- **Modules only tests import**: a helper module outside the test directories, such as `testing/fixtures.py`, that only
  test modules import, directly or through other such modules, serves the tests, so its calls are test code.

  ```python ok file=testing/fixtures.py
  def make_config(rev: str = "main") -> str:
      return rev


  def default_config() -> str:
      return make_config()
  ```

  ```python context file=tests/test_config.py
  from testing.fixtures import default_config, make_config


  def test_config() -> None:
      assert make_config(rev="v1") != default_config()
  ```

- **Parameters that inject a collaborator**: a defaulted parameter is a seam where a test swaps in a stand-in, which is
  a design choice rather than test-only plumbing. A collaborator is a function, a class, a module, or an instance built
  by calling a class. It is exempt in these cases:

  - a `None` default, when the body calls the parameter, such as `hook(address)`, or when the annotation, unwrapped from
    `Optional[...]` and `... | None`, names a class, Protocol, module or callable that is not a data type, and the body
    calls the parameter or reads an attribute of it. A `None` default alone does not exempt a parameter;
  - any other default that is a name, an attribute or a call resolving to a collaborator, when the body calls the
    parameter or reads an attribute of it, such as `mailer.send(address)`. A name bound to a literal, such as a `dict`,
    or to a call of a data constructor, such as `dict()`, is data, and so is a name that cannot be resolved.

  The data types are the builtins, such as `dict`, `list` and `str`, and the collection types of
  [`collections.abc`](https://docs.python.org/3/library/collections.abc.html#module-collections.abc) and `typing`, such
  as `Mapping` and `Sequence`. A parameter annotated `bool`, `int`, `float`, `str`, `bytes`, `complex` or `Literal[...]`
  stays reported, and so does one annotated with an enum, which the project defines or imports, as `Mode`,
  `Mode | None`, `Optional[Mode]` or `module.Mode`. So does one whose default is an enum member, such as `Mode.FAST`.

  ```python context file=app/service.py
  from app.mail import send


  def notify(address: str) -> None:
      send(address)
  ```

  ```python ok file=app/mail.py
  from collections.abc import Callable


  def send(address: str, hook: Callable[[str], None] | None = None) -> None:
      if hook is not None:
          hook(address)
  ```

  ```python context file=tests/test_mail.py
  from app.mail import send


  def test_send() -> None:
      send("a@b.c", hook=print)
  ```

  A data parameter the body only passes on, without calling it or reading an attribute, is still reported:

  ```python context file=app/service.py
  from app.mail import send


  def notify(address: str) -> None:
      send(address)
  ```

  ```python report file=app/mail.py
  from collections.abc import Mapping


  def send(address: str, options: Mapping[str, bool] | None = None) -> str:
      return address
  ```

  ```python context file=tests/test_mail.py
  from app.mail import send


  def test_send() -> None:
      send("a@b.c", options={})
  ```

- **Calls that unpack `*args` or `**kwargs`**: the unpacked values may pass the parameter.

  ```python context file=app/service.py
  from app.mail import send


  def notify(address: str, options: dict[str, bool]) -> None:
      send(address, **options)
  ```

  ```python ok file=app/mail.py
  def send(address: str, urgent: bool = False) -> None:
      print(address, urgent)
  ```

  ```python context file=tests/test_mail.py
  from app.mail import send


  def test_send() -> None:
      send("a@b.c", urgent=True)
  ```

### Fix

The rule has no fix. Removing the parameter changes the signature every caller sees, and the tests that pass it would
need the variation moved elsewhere first. The author decides where the variation belongs.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
