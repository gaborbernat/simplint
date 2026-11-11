Flags `re.match()` references, which `re.prefixmatch()` names from Python 3.15.

## Why is this bad?

Python 3.15 soft deprecates `re.match()` in favor of `re.prefixmatch()`, which says what the function does: it tries the
pattern at the start of the string. Other regular expression libraries call that operation a search, so `match` reads
differently to readers who know them. `re.match()` keeps working and is not planned for removal.

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

- **Pattern methods**: `pattern.match()` is not reported, since the receiver's type is not known and a method of that
  name may belong to any class.

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

- **Truth tests with a plain-text pattern Ruff rewrites**:
  [RUF055](https://docs.astral.sh/ruff/rules/unnecessary-regular-expression/) rewrites a `re.match` call whose pattern
  has no regular expression metacharacter, and whose result is only tested for truth or compared to `None`, to
  `str.startswith`, so this rule leaves that call to Ruff.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python ok
  import re


  def is_version(text):
      return bool(re.match("v", text))
  ```

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

- **[Ruff](https://github.com/astral-sh/ruff):** variation of preview rule
  [RUF055](https://docs.astral.sh/ruff/rules/unnecessary-regular-expression/), which rewrites `re.match` with a
  plain-string pattern to `str.startswith` and which this rule leaves to it; the rename for other patterns is open issue
  [#24545](https://github.com/astral-sh/ruff/issues/24545).
- [flake8-tergeo](https://github.com/SAP/flake8-tergeo) reports the same `re.match` calls as
  [FTP149](https://github.com/SAP/flake8-tergeo/blob/main/docs/checks.md#ftp149), with no fix.
