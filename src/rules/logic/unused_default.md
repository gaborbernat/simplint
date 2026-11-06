Flags a parameter default that every call of the function in the run passes explicitly, by position or by keyword.

## Why is this bad?

A default that no call relies on documents a value nobody uses. A reader looks at the signature to learn what the
function falls back to, and the fallback misleads once every caller supplies its own value.

## Example

```python report
def connect(host, timeout=30):
    return host, timeout


connect("db", 5)
```

Use instead:

```python ok
def connect(host, timeout):
    return host, timeout


connect("db", 5)
```

## Configuration

- [`project-kind`](https://simplint.readthedocs.io/en/latest/configuration/project.html#library-or-application): a
  library's public functions are left alone, since code outside the project may call them without the argument.

  ```toml config
  [tool.simplint]
  project-kind = "application"
  ```

  ```python report
  def connect(host, timeout=30):
      return host, timeout


  connect("db", 5)
  ```

## What it does

Calls are counted across the whole run, with names resolved through imports. A call that passes an argument for a
parameter by position counts for it, and so does one that passes it by keyword.

### Active on

- **Module-level functions called in the run**: each call in the defining file, and each call from another file that
  imports the function through its module.

  ```python report
  def connect(host, timeout=30):
      return host, timeout


  connect("db", 5)
  connect("cache", timeout=2)
  ```

  ```python fixed
  def connect(host, timeout):
      return host, timeout


  connect("db", 5)
  connect("cache", timeout=2)
  ```

- **Keyword-only parameters**: a parameter after `*` is passed by keyword only, so every call names it.

  ```python report
  def retry(count, *, delay=1):
      return count, delay


  retry(3, delay=2)
  ```

  ```python fixed
  def retry(count, *, delay):
      return count, delay


  retry(3, delay=2)
  ```

- **Calls from other files**: the calls of a function count wherever they are, so a default is reported only when no
  file in the run leaves it out.

  ```python report file=lib/core.py
  def connect(host, timeout=30):
      return host, timeout
  ```

  ```python context file=app/main.py
  from lib.core import connect


  connect("db", 5)
  ```

- **Methods called through `self`**: a method of a module-level class without bases, when the run reads its name only as
  `self.name(...)` calls inside that class. Another `obj.name` read anywhere in the run, or a subclass of the class,
  leaves the method alone, since the rule cannot tell which receiver it is.

  ```python report
  class Client:
      def send(self, data, flush=False):
          return data, flush

      def push(self, data):
          return self.send(data, True)
  ```

  ```python fixed
  class Client:
      def send(self, data, flush):
          return data, flush

      def push(self, data):
          return self.send(data, True)
  ```

### Exceptions

- **Functions with no call in the run**: there is nothing to pass the default, so the rule has no evidence either way.

  ```python ok
  def connect(host, timeout=30):
      return host, timeout
  ```

- **Calls that unpack `*args` or `**kwargs`**: the unpacked values can supply any parameter.

  - `*args`: the call unpacks a sequence into positional arguments.

    ```python ok
    def connect(host, timeout=30):
        return host, timeout


    connect("db", *extra)
    connect("db", 5)
    ```

  - `**kwargs`: the call unpacks a mapping into keyword arguments.

    ```python ok
    def connect(host, timeout=30):
        return host, timeout


    connect("db", **options)
    connect("db", 5)
    ```

- **Functions used other than as a call**: passed as a value, registered by a decorator, or named by a dotted string or
  an entry point, where the calls that reach it are not all visible.

  ```python ok
  def connect(host, timeout=30):
      return host, timeout


  connect("db", 5)
  handlers = [connect]
  ```

- **Decorated functions**: a decorator may register or wrap the function, so its calls may not be the ones written.

  ```python ok
  from functools import lru_cache


  @lru_cache
  def connect(host, timeout=30):
      return host, timeout


  connect("db", 5)
  ```

- **Dunder methods**: `__init__` and the other dunders are called by Python itself, not by name.

  ```python ok
  class Client:
      def __init__(self, timeout=30):
          self.timeout = timeout


  Client(5)
  ```

- **Public functions of a library**: outside code may call them without the argument.

  ```toml config
  [tool.simplint]
  project-kind = "library"
  api-modules = ["lib.core"]
  ```

  ```python ok file=lib/core.py
  __all__ = ["connect"]


  def connect(host, timeout=30):
      return host, timeout
  ```

  ```python context file=app/main.py
  from lib.core import connect


  connect("db", 5)
  ```

- **Methods of classes with bases, or whose name is read elsewhere**: the calls of a method through `self` may be
  overridden in a subclass, or the name may be read through another object.

  ```python ok
  class Base:
      pass


  class Client(Base):
      def send(self, data, flush=False):
          return data, flush

      def push(self, data):
          return self.send(data, True)
  ```

### Fix

The fix deletes the default, from its `=` to the end of its value. It is offered only when the signature stays valid: a
keyword-only parameter always qualifies, and a positional one only when no parameter before it has a default. Removing
the default of `first` in `def pair(first=1, second=2)` lets `second` qualify on the next pass, so the fixed output
below drops both.

**Unsafe fix:** the fix relies on the calls the run shows, so a call from outside the run may break.

```python report
def pair(first=1, second=2):
    return first, second


pair(1, 2)
```

```python fixed
def pair(first, second):
    return first, second


pair(1, 2)
```

## Other tools

- [refurb](https://github.com/dosisod/refurb) reports call arguments that equal the default as
  [FURB120](https://github.com/dosisod/refurb/blob/master/docs/checks.md#furb120-use-implicit-default). This rule
  removes the default from the signature instead, when every call passes it.
