Flags a `ContextVar.set()` token that a `try` block resets in its `finally` clause, which a `with` block does on exit
from Python 3.14.

## Why is this bad?

Since Python 3.14 the token `ContextVar.set()` returns is a context manager that resets the variable when the block
ends. The `try`/`finally` pair repeats that by hand, and leaves a name in scope that nothing else reads.

## Example

```toml config
[project]
requires-python = ">=3.14"
```

```python report
import contextvars

request_id = contextvars.ContextVar("request_id")


def handle(value):
    token = request_id.set(value)
    try:
        return process()
    finally:
        request_id.reset(token)
```

Use instead:

```python ok
import contextvars

request_id = contextvars.ContextVar("request_id")


def handle(value):
    with request_id.set(value):
        return process()
```

## Configuration

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python);
  see [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

  ```toml config
  [project]
  requires-python = ">=3.13"
  ```

  ```python ok
  import contextvars

  request_id = contextvars.ContextVar("request_id")


  def handle(value):
      token = request_id.set(value)
      try:
          return process()
      finally:
          request_id.reset(token)
  ```

## What it does

Reports an assignment `token = variable.set(value)` followed directly by a `try` whose only clause is a `finally` that
holds `variable.reset(token)`, with no `except` and no `else`. `variable` is a name the file assigns a
`contextvars.ContextVar(...)` call to, or a parameter annotated as `ContextVar`.

### Active on

```toml config
[project]
requires-python = ">=3.14"
```

```python report
import contextvars

request_id = contextvars.ContextVar("request_id")
token = request_id.set("one")
try:
    print(request_id.get())
finally:
    request_id.reset(token)
```

```python fixed
import contextvars

request_id = contextvars.ContextVar("request_id")
with request_id.set("one"):
    print(request_id.get())
```

### Exceptions

- **A token read outside the reset**: the block would take the name's last use away.

  ```toml config
  [project]
  requires-python = ">=3.14"
  ```

  ```python ok
  import contextvars

  request_id = contextvars.ContextVar("request_id")
  token = request_id.set("one")
  try:
      print(request_id.get())
  finally:
      request_id.reset(token)
  print(token)
  ```

- **A `finally` with more than the reset**: the statements after the reset would run inside the block.

  ```toml config
  [project]
  requires-python = ">=3.14"
  ```

  ```python ok
  import contextvars

  request_id = contextvars.ContextVar("request_id")
  token = request_id.set("one")
  try:
      print(request_id.get())
  finally:
      request_id.reset(token)
      print("done")
  ```

- **A reset on another variable**: the token belongs to the variable it was set on.

  ```toml config
  [project]
  requires-python = ">=3.14"
  ```

  ```python ok
  import contextvars

  request_id = contextvars.ContextVar("request_id")
  user = contextvars.ContextVar("user")
  token = request_id.set("one")
  try:
      print(request_id.get())
  finally:
      user.reset(token)
  ```

- **A receiver not known to be a `ContextVar`**: an attribute, an unannotated parameter, or an object of another class,
  whose `set` may return something that is not a context manager.

  ```toml config
  [project]
  requires-python = ">=3.14"
  ```

  ```python ok
  class Settings:
      def set(self, value):
          old, self.value = self.value, value
          return old

      def reset(self, old):
          self.value = old


  settings = Settings()
  token = settings.set("one")
  try:
      print(settings.value)
  finally:
      settings.reset(token)
  ```

- **Target versions before 3.14**: the token is not a context manager there, and `with` raises `TypeError`.

  ```toml config
  [project]
  requires-python = ">=3.13"
  ```

  ```python ok
  import contextvars

  request_id = contextvars.ContextVar("request_id")
  token = request_id.set("one")
  try:
      print(request_id.get())
  finally:
      request_id.reset(token)
  ```

### Fix

The fix is safe and reports without a fix when a comment sits in the lines it would remove, which would lose the
comment, when the assignment shares its line with an earlier statement, or when the line before it ends with a backslash
that joins the two lines. A module-level token that another module of the run reads, through an import, a star import or
an attribute, or that `__all__` lists, keeps its report without a fix, since removing the assignment would break those
readers:

```toml config
[project]
requires-python = ">=3.14"
```

```python report
import contextvars

request_id = contextvars.ContextVar("request_id")


def handle(value):
    # fmt: off
    started = True; token = request_id.set(value)
    # fmt: on
    try:
        return process()
    finally:
        request_id.reset(token)
```

```python fixed
import contextvars

request_id = contextvars.ContextVar("request_id")


def handle(value):
    # fmt: off
    started = True; token = request_id.set(value)
    # fmt: on
    try:
        return process()
    finally:
        request_id.reset(token)
```

```toml config
[project]
requires-python = ">=3.14"
```

```python report
import contextvars

request_id = contextvars.ContextVar("request_id")


def handle(value):
    # fmt: off
    started = True; \
    token = request_id.set(value)
    # fmt: on
    try:
        return process()
    finally:
        request_id.reset(token)
```

```python fixed
import contextvars

request_id = contextvars.ContextVar("request_id")


def handle(value):
    # fmt: off
    started = True; \
    token = request_id.set(value)
    # fmt: on
    try:
        return process()
    finally:
        request_id.reset(token)
```

```toml config
[project]
requires-python = ">=3.14"
```

```python context file=user.py
from state import token
```

```python report file=state.py
import contextvars

request_id = contextvars.ContextVar("request_id")
token = request_id.set("one")
try:
    print(request_id.get())
finally:
    request_id.reset(token)
```

```python fixed file=state.py
import contextvars

request_id = contextvars.ContextVar("request_id")
token = request_id.set("one")
try:
    print(request_id.get())
finally:
    request_id.reset(token)
```

A star import of the module counts as a read of the name, so the report stays without a fix:

```toml config
[project]
requires-python = ">=3.14"
```

```python context file=user.py
from state import *

print(token)
```

```python report file=state.py
import contextvars

request_id = contextvars.ContextVar("request_id")
token = request_id.set("one")
try:
    print(request_id.get())
finally:
    request_id.reset(token)
```

```python fixed file=state.py
import contextvars

request_id = contextvars.ContextVar("request_id")
token = request_id.set("one")
try:
    print(request_id.get())
finally:
    request_id.reset(token)
```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
