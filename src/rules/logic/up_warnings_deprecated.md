Flags functions and classes whose first statement after an optional docstring calls
[`warnings.warn()`](https://docs.python.org/3/library/warnings.html#warnings.warn) with
[`DeprecationWarning`](https://docs.python.org/3/builtins/exceptions.html#DeprecationWarning).

## Why is this bad?

Since Python 3.13, [`@warnings.deprecated`](https://docs.python.org/3/library/warnings.html#warnings.deprecated) marks
the object for [type checkers](https://typing.python.org/en/latest/spec/directives.html#deprecated) as well as warning
at runtime. A type checker can flag a deprecated call on a path that tests never execute. By default, the decorator
emits `DeprecationWarning` when a function is called or a class is instantiated or subclassed.

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

The fix is unsafe because the decorator can change when and where the warning appears.

- A warning in a class body runs when Python defines the class. The decorator warns on instantiation or subclassing.
- The fix drops the arguments after the message, including `stacklevel`. In a regular function,
  `warnings.warn(..., stacklevel=2)` and the default decorator both point at the caller. Other levels can point
  elsewhere.

The fix requires a one-line string literal because Python evaluates the decorator at definition time. An f-string could
refer to a parameter that has no value yet. The warning must also occupy its own lines, without comments, and must be
called through the module, for example `warnings.warn(...)` or `w.warn(...)` after `import warnings as w`.

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

- Ruff's [B028](https://docs.astral.sh/ruff/rules/no-explicit-stacklevel/) reports a `warnings.warn()` call without an
  explicit `stacklevel`.
- [flake8-tergeo](https://github.com/SAP/flake8-tergeo) reports functions that call `warnings.warn` where
  `warnings.deprecated` fits, as [FTP118](https://github.com/SAP/flake8-tergeo/blob/main/docs/checks.md#ftp118), with no
  fix.
