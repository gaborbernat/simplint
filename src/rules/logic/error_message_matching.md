Flags an `except` handler that tells exceptions apart by their text, by comparing or searching `str(exc)`, `exc.args` or
`exc.message`.

## Why is this bad?

The text of an exception is a message for people, and it changes with the wording, the translation and the library
version. CPython makes no promise about it: core developers
[keep messages out of the API](https://bugs.python.org/msg227794) and
[advise against relying on them](https://discuss.python.org/t/pep-387-including-exception-messages-in-the-backward-compatibility-policy/35850/20).
A check on the text stops matching without any error, and the handler then takes the wrong branch.

A specific exception type says what happened, and it does not change with the text.
[PEP 3151](https://peps.python.org/pep-3151/#lack-of-fine-grained-exceptions) added subclasses such as
[`TimeoutError`](https://docs.python.org/3/builtins/exceptions.html#TimeoutError) and
[`FileNotFoundError`](https://docs.python.org/3/builtins/exceptions.html#FileNotFoundError), so handlers no longer
inspect a caught [`OSError`](https://docs.python.org/3/builtins/exceptions.html#OSError). Where no subclass exists, a
data attribute such as `errno` is the value to test.

## Example

```python report
try:
    data = load(path)
except OSError as exc:
    if "timed out" in str(exc):
        data = load(path, retries=3)
    else:
        raise
```

Use instead:

```python ok
try:
    data = load(path)
except TimeoutError:
    data = load(path, retries=3)
```

## What it does

Each case is checked inside the body of an `except ... as exc:` handler. Test modules are left alone, since their
asserts may check the messages the code raises.

### Active on

- **Comparisons and `in` tests with the text**: an operand that is `str(exc)`, `exc.args`, `exc.args[N]` or
  `exc.message`, in an `if` or `elif` condition or anywhere else in the handler.

  ```python report
  def run(exc_source):
      try:
          exc_source()
      except OSError as exc:
          if "timed out" in str(exc):
              return None
          if exc.args[0] == "x":
              return ""
          return exc
  ```

- **`str` methods that search the text**: `startswith`, `endswith`, `find`, `index` and `count` called on `str(exc)`.

  ```python report
  def run(exc_source):
      try:
          exc_source()
      except OSError as exc:
          if str(exc).startswith("timed"):
              return None
          return exc
  ```

- **`re` functions matching the text**: [`re.search`](https://docs.python.org/3/library/re.html#re.search),
  [`re.match`](https://docs.python.org/3/library/re.html#re.match) and
  [`re.fullmatch`](https://docs.python.org/3/library/re.html#re.fullmatch) with `str(exc)` as an argument.

  ```python report
  import re


  def run(exc_source):
      try:
          exc_source()
      except OSError as exc:
          if re.search("gone", str(exc)):
              return None
          return exc
  ```

- **`match` subjects**: a `match` statement whose subject is one of the text forms.

  ```python report
  def run(exc_source):
      try:
          exc_source()
      except OSError as exc:
          match str(exc):
              case "missing":
                  return None
          return exc
  ```

### Exceptions

- **The text in logging, formatting or re-raising**: a message built for a person does not pick a branch.

  ```python ok
  import logging

  logger = logging.getLogger(__name__)


  def run(exc_source):
      try:
          exc_source()
      except OSError as exc:
          logger.error("failed: %s", str(exc))
          print(f"failed: {exc}")
          raise
  ```

- **Attributes that hold data**: `exc.errno` and other fields are values, not text.

  ```python ok
  def run(exc_source):
      try:
          exc_source()
      except OSError as exc:
          if exc.errno == 2:
              return None
          raise
  ```

- **`assert` statements that name the exception**: Ruff's
  [pytest-assert-in-except](https://docs.astral.sh/ruff/rules/pytest-assert-in-except/) (PT017) reports an `assert` in
  an `except` body that names the exception, so this rule leaves the comparison inside it to Ruff.

  ```python ok
  def run(exc_source):
      try:
          exc_source()
      except OSError as exc:
          assert "gone" in str(exc)
  ```

- **Comparisons of two exceptions' text**: every operand is the text of an exception, such as `str(exc) == str(other)`.
  Both sides are text the runtime produces, so no fixed wording is written into the code that could drift. A name is an
  exception when each of its bindings in scope is an `except` clause name, or a parameter annotated with an exception
  type: a builtin one, a class of the run or one it imports from there, alone, in a union with `None`, in
  `Optional[...]`, or in a string annotation of those.

  ```python ok
  def replayed(target, error: OSError):
      try:
          target()
      except OSError as replay_error:
          return type(replay_error) is type(error) and replay_error.args == error.args
  ```

- **Test modules**: tests may assert on the messages the code raises.

  ```python ok file=test_errors.py
  def test_load():
      try:
          load()
      except OSError as exc:
          assert "gone" in str(exc)
  ```

- **`pytest.raises` with `match`**: the `match` argument of `pytest.raises` is a pattern the test expects, not a branch.

  ```python ok
  import pytest


  def test_load():
      with pytest.raises(OSError, match="gone"):
          load()
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rule
  [PT017](https://docs.astral.sh/ruff/rules/pytest-assert-in-except/), which reports an `assert` on the caught exception
  and which this rule leaves to it; branching on the message text is not requested yet.
