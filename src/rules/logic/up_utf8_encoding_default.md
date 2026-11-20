Flags `encoding="utf-8"` arguments on text reads and writes, which [PEP 686](https://peps.python.org/pep-0686/) makes
the default in Python 3.15.

## Why is this bad?

Python 3.15 enables [UTF-8 mode](https://docs.python.org/3/library/os.html#utf8-mode) by default. In that mode,
[`open()`](https://docs.python.org/3/builtins/functions.html#open) and the `pathlib` text methods already use UTF-8.
Code that requires UTF-8 even when the mode is disabled should keep the explicit encoding, as
[PEP 686](https://peps.python.org/pep-0686/#backward-compatibility) recommends.

## Example

```toml config
[project]
requires-python = ">=3.15"
```

```python report
from pathlib import Path


def read_config(path):
    with open(path, encoding="utf-8") as handle:
        return handle.read()


def write_log(path, text):
    Path(path).write_text(text, encoding="utf-8")
```

Use instead:

```python ok
from pathlib import Path


def read_config(path):
    with open(path) as handle:
        return handle.read()


def write_log(path, text):
    Path(path).write_text(text)
```

## Conflicts with Ruff

Ruff's preview rule [PLW1514](https://docs.astral.sh/ruff/rules/unspecified-encoding/) requests an explicit encoding.
Its fix adds the keyword this rule removes. When both rules apply to a file targeting Python 3.15 or later, simplint
stops before linting. Ignore either rule for that file.

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
  def read_config(path):
      with open(path, encoding="utf-8") as handle:
          return handle.read()
  ```

## What it does

### Active on

- **Calls of the builtin `open()`, [`io.open()`](https://docs.python.org/3/library/io.html#io.open) and
  [`io.TextIOWrapper()`](https://docs.python.org/3/library/io.html#io.TextIOWrapper), and of `open`,
  [`read_text`](https://docs.python.org/3/library/pathlib.html#pathlib.Path.read_text) and
  [`write_text`](https://docs.python.org/3/library/pathlib.html#pathlib.Path.write_text) methods**, when the call passes
  a string literal for `encoding=`. Recognized names are `utf_8`, `utf8`, `u8`, `utf`, `utf8_ucs2`, `utf8_ucs4` and
  `cp65001` after lowercasing and replacing hyphens and spaces with underscores. The methods are matched by name, since
  their receiver's type is not known, so a method of another class with the same name is reported too. **Unsafe fix:**
  the keyword is removed.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python report
  def read_config(path):
      with open(path, encoding="UTF8") as handle:
          return handle.read()
  ```

  ```python fixed
  def read_config(path):
      with open(path) as handle:
          return handle.read()
  ```

### Fix

The fix is unsafe. With `PYTHONUTF8=0` or `-X utf8=0`, the default encoding can be locale-specific, so removing the
argument can change the bytes written or cause a decoding error. A same-named method on another class may also use a
different default. Omitting the keyword can trigger
[`EncodingWarning`](https://docs.python.org/3/builtins/exceptions.html#EncodingWarning) under `-X warn_default_encoding`
or `PYTHONWARNDEFAULTENCODING`, including in UTF-8 mode.

simplint reports the keyword without a fix if the call contains a comment.

A parenthesized positional argument stays a valid call when the keyword after it is removed.

```toml config
[project]
requires-python = ">=3.15"
```

```python report
def read_config(path):
    with open(("config.txt"), encoding="utf-8") as handle:
        return handle.read()
```

```python fixed
def read_config(path):
    with open("config.txt") as handle:
        return handle.read()
```

### Exceptions

- **Targets before Python 3.15.** The rule starts at 3.15, when UTF-8 mode became the default across locales.

  ```toml config
  [project]
  requires-python = ">=3.14"
  ```

  ```python ok
  def read_config(path):
      with open(path, encoding="utf-8") as handle:
          return handle.read()
  ```

- **Other encodings**: `utf-8-sig` writes a byte order mark and `latin-1` decodes other bytes, so neither is the
  default.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python ok
  def read_config(path):
      with open(path, encoding="utf-8-sig") as handle:
          return handle.read()
  ```

- **Module functions such as [`tarfile.open()`](https://docs.python.org/3/library/tarfile.html#tarfile.open)**: their
  `encoding` argument has its own meaning, so they are not reported.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python ok
  import tarfile


  def read_archive(path):
      return tarfile.open(path, encoding="utf-8")
  ```

- **The `encoding` passed positionally**: only the keyword form is reported.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python ok
  def read_config(path):
      with open(path, "r", -1, "utf-8") as handle:
          return handle.read()
  ```

## Other tools

- [Ruff issue #29326](https://github.com/astral-sh/ruff/issues/29326) tracks the behavior of PLW1514 on Python 3.15.
- flake8-tergeo's [FTP140](https://github.com/SAP/flake8-tergeo/blob/main/docs/checks.md#ftp140) checks explicit UTF-8
  encodings in any function call on Python 3.15 and later.
