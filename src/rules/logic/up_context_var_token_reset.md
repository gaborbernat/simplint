Flags a `try` block whose `finally` clause resets the token from
[`ContextVar.set()`](https://docs.python.org/3/library/contextvars.html#contextvars.ContextVar.set).

## Why is this bad?

Since Python 3.14, [the token](https://docs.python.org/3/library/contextvars.html#contextvars.Token) is a context
manager. `with var.set(value):` restores the previous value when the block exits, including when its body raises an
exception.

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

The assignment `token = variable.set(value)` must immediately precede the `try` block. Its only clause must be a
`finally` containing just `variable.reset(token)`. Every binding of `variable` must be a call to
[`ContextVar`](https://docs.python.org/3/library/contextvars.html#contextvars.ContextVar) or a parameter annotated as
one.

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

- **Another use of the token.** The `with` statement removes the assignment that provides the token.

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

- **Other statements in the `finally` clause.** Replacing the clause with an automatic reset would drop those
  statements.

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

- **Target versions before 3.14**: the token is not a context manager there, and `with` raises
  [`TypeError`](https://docs.python.org/3/builtins/exceptions.html#TypeError).

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

simplint marks this rewrite as safe. It assumes that the receiver of `reset()` still refers to the same variable when
the block exits. Rebinding that name inside the block can make the original reset fail, while the context manager still
resets the original variable.

The fix replaces the assignment and cleanup with a `with` statement. simplint withholds it if an edit would remove a
comment, the assignment follows another statement on the same line or continues the previous line, or the `try` body
starts on the `try` line. It also keeps a module-level assignment if `__all__` exports the token or another analyzed
module reads it through an import, star import or attribute.

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

A star import can expose the token to another analyzed module.

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
