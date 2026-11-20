Flags reads of [`re.match`](https://docs.python.org/3/library/re.html#re.match) on Python 3.15 and later.

## Why is this bad?

[`re.prefixmatch()`](https://docs.python.org/3/library/re.html#re.prefixmatch) checks for a match at the beginning of
the string. [`re.search()`](https://docs.python.org/3/library/re.html#re.search) can find a match anywhere. Python 3.15
[soft-deprecated the name `re.match()`](https://docs.python.org/3/library/re.html#prefixmatch-vs-match). It remains an
alias for `re.prefixmatch()`.

## Example

```toml config
[project]
requires-python = ">=3.15"
```

```python report
import re


def parse_version(text):
    found = re.match(r"\d+\.\d+", text)
    return found.group(0) if found else None
```

Use instead:

```python ok
import re


def parse_version(text):
    found = re.prefixmatch(r"\d+\.\d+", text)
    return found.group(0) if found else None
```

## Configuration

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python);
  see [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

  ```toml config
  [tool.ruff]
  target-version = "py315"
  ```

  ```python report
  import re


  def parse_version(text):
      return re.match(r"\d+\.\d+", text)
  ```

## What it does

### Active on

- **`re.match` references**, where `re` is the module. **Safe fix:** the attribute is renamed to `prefixmatch`.
  `re.prefixmatch` is the same function as `re.match` from Python 3.15, so the rename keeps the behavior.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python report
  import re


  def parse_version(text):
      found = re.match(r"\d+\.\d+", text)
      return found.group(0) if found else None
  ```

  ```python fixed
  import re


  def parse_version(text):
      found = re.prefixmatch(r"\d+\.\d+", text)
      return found.group(0) if found else None
  ```

### Exceptions

- **Targets before Python 3.15**: `re.prefixmatch` does not exist there.

  ```toml config
  [tool.ruff]
  target-version = "py314"
  ```

  ```python ok
  import re


  def parse_version(text):
      return re.match(r"\d+\.\d+", text)
  ```

- **Pattern methods**: [`pattern.match()`](https://docs.python.org/3/library/re.html#re.Pattern.match) is not reported,
  since the receiver's type is not known and a method of that name may belong to any class.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python ok
  import re

  PARSER = re.compile(r"\d+\.\d+")


  def parse_version(text):
      return PARSER.match(text)
  ```

- **Assignments and `del`**: `re.match = ...` patches the module's `match` name, which a rename would leave in place.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python ok
  import re


  def patch(replacement):
      re.match = replacement
  ```

- **Names imported from `re`**: `from re import match` is not reported, since renaming it would also rename its uses.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python ok
  from re import match


  def parse_version(text):
      return match(r"\d+\.\d+", text)
  ```

- **Plain-text truth tests.** simplint leaves two-argument calls with a plain string or bytes pattern to Ruff's
  [RUF055](https://docs.astral.sh/ruff/rules/unnecessary-regular-expression/) when the result is tested for truth or
  compared with `is None` or `is not None`. This exception also applies when RUF055 is disabled.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python ok
  import re


  def is_version(text):
      return bool(re.match("v", text))
  ```

- **Patterns anchored at their end, when [`anchored-regex-match`](SMP550.html) is selected**: that rule turns the call
  into [`re.fullmatch()`](https://docs.python.org/3/library/re.html#re.fullmatch), which a rename to `prefixmatch` would
  only delay.

- **A name also bound to another module**: `re` may be another module in that file, which may have no `prefixmatch`, so
  the `re.match` of a `try` import fallback is left alone.

  ```toml config
  [tool.ruff]
  target-version = "py315"
  ```

  ```python ok
  try:
      import regex as re
  except ImportError:
      import re


  def first_version(text):
      return re.match(r"\d+", text)
  ```

## Other tools

- Ruff's preview rule [RUF055](https://docs.astral.sh/ruff/rules/unnecessary-regular-expression/) can replace a
  plain-text prefix test with `startswith()`. [Ruff issue #24545](https://github.com/astral-sh/ruff/issues/24545)
  proposes renaming other calls to `prefixmatch()`.
- [flake8-tergeo](https://github.com/SAP/flake8-tergeo) reports the same `re.match` calls as
  [FTP149](https://github.com/SAP/flake8-tergeo/blob/main/docs/checks.md#ftp149), with no fix.
