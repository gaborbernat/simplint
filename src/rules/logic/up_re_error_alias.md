Flags reads of `re.error` on Python 3.13 and later.

## Why is this bad?

Python 3.13 renamed the exception raised for an invalid pattern to
[`re.PatternError`](https://docs.python.org/3/library/re.html#re.PatternError). The old spelling `re.error` remains a
supported alias for the same class.

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

  A `del` of the name does not bind it to another module, so the reference before it is still reported:

  ```toml config
  [project]
  requires-python = ">=3.13"
  ```

  ```python report
  import re

  ERROR = re.error
  del re
  ```

  ```python fixed
  import re

  ERROR = re.PatternError
  del re
  ```

### Exceptions

- **Assignments and deletions.** These change the module attribute. Editing `re.error` and editing `re.PatternError` can
  affect different callers.

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

- **A name bound to multiple modules.** For example, after a fallback import, `re` may refer to the third-party `regex`
  module.

  ```toml config
  [tool.ruff]
  target-version = "py314"
  ```

  ```python ok
  try:
      import regex as re
  except ImportError:
      import re


  def compile_pattern(text):
      try:
          return re.compile(text)
      except re.error:
          return None
  ```
