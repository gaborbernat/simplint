Flags functions and classes that open with a
[`warnings.warn()`](https://docs.python.org/3/library/warnings.html#warnings.warn) call of
[`DeprecationWarning`](https://docs.python.org/3/builtins/exceptions.html#DeprecationWarning), which
[`@warnings.deprecated`](https://docs.python.org/3/library/warnings.html#warnings.deprecated)
([PEP 702](https://peps.python.org/pep-0702/)) replaces from Python 3.13.

## Why is this bad?

[`warnings.deprecated()`](https://docs.python.org/3/library/warnings.html#warnings.deprecated) marks a function or class
as deprecated where both the runtime and the tools see it. [PEP 702](https://peps.python.org/pep-0702/) argues that
runtime warnings are often insufficient: they get silenced, and a call on an untested path never warns. With the
decorator, [type checkers](https://typing.python.org/en/latest/spec/directives.html#deprecated) report each use, and
editors show it as the code is written.

The decorator still emits the `DeprecationWarning` on call, instantiation or subclassing. It points the warning at the
caller itself, where a `warnings.warn()` call needs a `stacklevel` that each project has to get right.

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
  since `@warnings.deprecated` raises [`TypeError`](https://docs.python.org/3/builtins/exceptions.html#TypeError) on the
  object [`@property`](https://docs.python.org/3/builtins/functions.html#property) or
  [`@classmethod`](https://docs.python.org/3/builtins/functions.html#classmethod) returns.

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

A comment on the line of the call counts as well, since the fix would remove that line with it:

```toml config
[project]
requires-python = ">=3.13"
```

```python report
import warnings


def connect(host):
    warnings.warn("connect() is deprecated", DeprecationWarning)  # remove in 3.0
    return host
```

```python fixed
import warnings


def connect(host):
    warnings.warn("connect() is deprecated", DeprecationWarning)  # remove in 3.0
    return host
```

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

- **Another category**: a [`FutureWarning`](https://docs.python.org/3/builtins/exceptions.html#FutureWarning) or a user
  warning class is not a deprecation that the decorator marks.

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

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet. Released rule
  [B028](https://docs.astral.sh/ruff/rules/no-explicit-stacklevel/) reports the same call when it lacks `stacklevel` and
  adds one; this rule replaces the call with the decorator.
- [flake8-tergeo](https://github.com/SAP/flake8-tergeo) reports functions that call `warnings.warn` where
  `warnings.deprecated` fits, as [FTP118](https://github.com/SAP/flake8-tergeo/blob/main/docs/checks.md#ftp118), with no
  fix.
