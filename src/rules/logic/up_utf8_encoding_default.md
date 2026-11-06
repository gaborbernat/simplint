Flags `encoding="utf-8"` arguments on text reads and writes, which Python 3.15 makes the default.

## Why is this bad?

Python 3.15 turns on UTF-8 mode by default, so `open()` and the `pathlib` text methods decode and encode UTF-8 when no
`encoding` is given. An argument that names the same encoding repeats the default, and a reader has to check whether it
changes anything. The default can be switched off with `PYTHONUTF8=0` or `-X utf8=0`, though, and then the argument is
what keeps the file in UTF-8.

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

Ruff's [unspecified-encoding](https://docs.astral.sh/ruff/rules/unspecified-encoding/) (PLW1514), a preview rule, asks
for the `encoding` argument this rule removes, and its fix adds `encoding="utf-8"` back. simplint stops with an error
before linting while both apply to a file that targets Python 3.15 or newer; ignore one of the two rules.

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

- **Calls of the builtin `open()`, `io.open()` and `io.TextIOWrapper()`, and of `open`, `read_text` and `write_text`
  methods**, when the call passes `encoding=` with a UTF-8 codec name: `utf-8`, `UTF-8`, `utf8`, `u8`, `utf_8` and the
  other spellings Python maps to UTF-8. The methods are matched by name, since their receiver's type is not known, so a
  method of another class with the same name is reported too. **Unsafe fix:** the keyword is removed.

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

The fix is unsafe because the default is not always UTF-8. With UTF-8 mode off, `open()` uses the locale's encoding,
which is ASCII under `LC_ALL=C`, so the file that the argument read as UTF-8 raises `UnicodeDecodeError` once the
argument is gone. Python 3.15 also still emits `EncodingWarning` for a call without `encoding` under
`-X warn_default_encoding` or `PYTHONWARNDEFAULTENCODING`, so a test suite that turns that warning into an error fails
after the fix.

No fix is offered when the call has a comment inside it, which the rewrite would drop.

### Exceptions

- **Targets before Python 3.15**: UTF-8 mode is not the default there, so the argument stays.

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

- **Module functions such as `tarfile.open()`**: their `encoding` argument has its own meaning, so they are not
  reported.

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

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rule
  [PLW1514](https://docs.astral.sh/ruff/rules/unspecified-encoding/) (preview), which asks for an explicit encoding, the
  opposite; open issue [#29326](https://github.com/astral-sh/ruff/issues/29326).
- [flake8-tergeo](https://github.com/SAP/flake8-tergeo) reports the same `encoding="UTF-8"` arguments as
  [FTP140](https://github.com/SAP/flake8-tergeo/blob/main/docs/checks.md#ftp140), with no fix.
