Flags functions and classes that open with a `warnings.warn()` call of `DeprecationWarning`, which
`@warnings.deprecated` replaces from Python 3.13.

## Why is this bad?

`warnings.deprecated()` is a decorator that marks a function, class or overload as deprecated. Type checkers report its
uses, and the decorator emits the `DeprecationWarning` when the object is called, instantiated or subclassed. A
`warnings.warn()` call in the body only warns at runtime, so a type checker cannot tell which code is deprecated, and
the warning's location is set by a `stacklevel` argument that each project has to get right.

## Example

```toml config
[project]
requires-python = ">=3.13"
```

```python report
import warnings


def connect(host):
    warnings.warn("connect() is deprecated, use open()", DeprecationWarning, stacklevel=2)
    return host
```

Use instead:

```python ok
import warnings


@warnings.deprecated("connect() is deprecated, use open()")
def connect(host):
    return host
```

## Configuration

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python);
  see [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

  ```toml config
  [tool.ruff]
  target-version = "py313"
  ```

  ```python report
  import warnings


  def connect(host):
      warnings.warn("connect() is deprecated, use open()", DeprecationWarning)
      return host
  ```

## What it does

### Active on

- **Functions, methods and classes whose first statement after the docstring calls `warnings.warn()` with
  `DeprecationWarning`**, passed as the second argument or as `category=`. The report sits on the call. **Unsafe fix:**
  the call is removed and a `@warnings.deprecated(...)` decorator takes its message, placed below any other decorator,
  since `@warnings.deprecated` raises `TypeError` on the object `@property` or `@classmethod` returns.

  ```toml config
  [project]
  requires-python = ">=3.13"
  ```

  ```python report
  import warnings


  class Client:
      """A client for the old protocol."""

      warnings.warn("Client is deprecated", DeprecationWarning, stacklevel=2)

      def send(self, data):
          return data
  ```

  ```python fixed
  import warnings


  @warnings.deprecated("Client is deprecated")
  class Client:
      """A client for the old protocol."""

      def send(self, data):
          return data
  ```

### Fix

The fix is unsafe, because the decorator does not warn where the call did:

- A class body warns when the class is defined. `@warnings.deprecated` on a class warns when an instance is created or a
  subclass is defined, so the warning moves from import time to those uses.
- A `stacklevel` of 2 points the warning at the caller in both forms. The default of 1 points it inside the function,
  which the decorator no longer does, since it always reports the caller.
- The decorator has its own `stacklevel`, so the call's `stacklevel` argument is dropped.

No fix is offered when the message is not a string literal on one line, since the decorator evaluates it where the
function is defined, where an f-string could read a parameter that does not exist yet. None either when the warning has
a comment, or when `warn` is imported by name, since the decorator needs the `warnings` module.

### Exceptions

- **A warning that is not the first statement**: the function may do work before it, which the decorator would change.

  ```toml config
  [project]
  requires-python = ">=3.13"
  ```

  ```python ok
  import warnings


  def connect(host):
      validate(host)
      warnings.warn("connect() is deprecated", DeprecationWarning)
      return host


  def validate(host):
      return host
  ```

- **Another category**: a `FutureWarning` or a user warning class is not a deprecation that the decorator marks.

  ```toml config
  [project]
  requires-python = ">=3.13"
  ```

  ```python ok
  import warnings


  def connect(host):
      warnings.warn("connect() changes", FutureWarning)
      return host
  ```

- **Targets before Python 3.13**: `warnings.deprecated` does not exist there.

  ```toml config
  [tool.ruff]
  target-version = "py312"
  ```

  ```python ok
  import warnings


  def connect(host):
      warnings.warn("connect() is deprecated", DeprecationWarning)
      return host
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
- [flake8-tergeo](https://github.com/SAP/flake8-tergeo) reports functions that call `warnings.warn` where
  `warnings.deprecated` fits, as [FTP118](https://github.com/SAP/flake8-tergeo/blob/main/docs/checks.md#ftp118), with no
  fix.
