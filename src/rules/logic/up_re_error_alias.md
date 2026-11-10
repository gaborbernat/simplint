Flags `re.error` references, which `re.PatternError` names from Python 3.13.

## Why is this bad?

`re.PatternError` is the name the standard library documents for the exception `re` raises on an invalid pattern. From
Python 3.13 it is the same class as `re.error`, so the new name says what the exception is. `re.error` is not deprecated
and keeps working.

## Example

```toml config
[project]
requires-python = ">=3.13"
```

```python report
import re


def compile_pattern(text):
    try:
        return re.compile(text)
    except re.error:
        return None
```

Use instead:

```python ok
import re


def compile_pattern(text):
    try:
        return re.compile(text)
    except re.PatternError:
        return None
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
  import re


  def compile_pattern(text):
      try:
          return re.compile(text)
      except re.error:
          return None
  ```

## What it does

### Active on

- **`re.error` references**, where `re` is the module. **Safe fix:** the attribute is renamed to `PatternError`.
  `re.PatternError is re.error` on Python 3.13 and later, so the rename keeps the behavior. A name imported from `re`,
  such as `from re import error`, is not reported.

  ```toml config
  [project]
  requires-python = ">=3.13"
  ```

  ```python report
  import re


  def compile_pattern(text):
      try:
          return re.compile(text)
      except re.error:
          return None
  ```

  ```python fixed
  import re


  def compile_pattern(text):
      try:
          return re.compile(text)
      except re.PatternError:
          return None
  ```

### Exceptions

- **An assignment or `del`**: it patches the module, and renamed it would patch `re.PatternError` while code that reads
  `re.error` keeps the original.

  ```toml config
  [project]
  requires-python = ">=3.13"
  ```

  ```python ok
  import re


  class LoggedError(re.PatternError):
      pass


  re.error = LoggedError
  ```

- **Targets before Python 3.13**: `re.PatternError` does not exist there.

  ```toml config
  [tool.ruff]
  target-version = "py312"
  ```

  ```python ok
  import re


  def compile_pattern(text):
      try:
          return re.compile(text)
      except re.error:
          return None
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
