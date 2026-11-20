Flags [`codecs.open()`](https://docs.python.org/3/library/codecs.html#codecs.open) calls, which the builtin
[`open()`](https://docs.python.org/3/builtins/functions.html#open) replaces.

## Why is this bad?

Python 3.14 [deprecates `codecs.open()`](https://docs.python.org/3/library/codecs.html#codecs.open). For ordinary text
encodings, the builtin `open()` accepts the encoding and error handler directly. Use `newline=""` to preserve line
endings when reading and writing. The two APIs split lines differently during iteration.
[PEP 400](https://peps.python.org/pep-0400/#textiowrapper-features) also describes how the builtin text stream reads
ahead to speed up small reads.

## Example

```toml config
[project]
requires-python = ">=3.10"
```

```python report
import codecs


def read_config(path):
    with codecs.open(path, "r", "utf-8") as handle:
        return handle.read()


def encoding_name():
    return codecs.lookup("utf-8").name
```

Use instead:

```python ok
import codecs


def read_config(path):
    with open(path, "r", encoding="utf-8", newline="") as handle:
        return handle.read()


def encoding_name():
    return codecs.lookup("utf-8").name
```

## Configuration

The rule applies to every supported target version.

## What it does

### Active on

- **Calls of `codecs.open`**, by its qualified name or by a name imported from `codecs`. **Unsafe fix:** the call
  becomes `open(...)` with `encoding=` and `newline=""`. The codecs stream also splits lines at characters such as form
  feed, U+2028, U+001C and U+0085. The builtin text stream recognizes carriage return, line feed and CRLF. Some codecs
  supported by `codecs.open()` are unavailable through `open()`.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import codecs


  def read_config(path):
      with codecs.open(path, "r", "utf-8") as handle:
          return handle.read()


  def encoding_name():
      return codecs.lookup("utf-8").name
  ```

  ```python fixed
  import codecs


  def read_config(path):
      with open(path, "r", encoding="utf-8", newline="") as handle:
          return handle.read()


  def encoding_name():
      return codecs.lookup("utf-8").name
  ```

### Fix

simplint reports the call without a fix in these cases.

- The encoding is omitted or `None`. In this case `codecs.open()` uses the builtin file object for the requested mode
  and ignores `errors`.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import codecs


  def read_text(path):
      return codecs.open(path).read()
  ```

- The mode is computed or contains `b`. The builtin `open()` rejects an encoding in binary mode.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import codecs


  def read_bytes(path):
      return codecs.open(path, "rb", "utf-8").read()
  ```

- A selected Ruff rule still flags the rewritten call after Ruff applies its fixes, for example
  [PTH123](https://docs.astral.sh/ruff/rules/builtin-open/).

- The call unpacks arguments with `*` or `**`, or a binding in the module or an enclosing scope would hide the builtin
  `open`.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import codecs


  def read(path, open=None):
      return codecs.open(path, "r", "utf-8").read()
  ```

  ```python fixed
  import codecs


  def read(path, open=None):
      return codecs.open(path, "r", "utf-8").read()
  ```

- The call contains a comment.

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import codecs


  def read_config(path):
      return codecs.open(
          path,  # the config file
          "r",
          "utf-8",
      ).read()
  ```

The fix preserves parentheses around `encoding`, `errors` and `buffering` values. It adds them when the new keyword
syntax requires them, as in `encoding=(name := value)`.

```toml config
[project]
requires-python = ">=3.14"
```

```python report
import codecs


def read(path):
    with codecs.open(path, "r", name := "utf-8") as handle:
        return name, handle.read()


def encoding_name():
    return codecs.lookup("utf-8").name
```

```python fixed
import codecs


def read(path):
    with open(path, "r", encoding=(name := "utf-8"), newline="") as handle:
        return name, handle.read()


def encoding_name():
    return codecs.lookup("utf-8").name
```

A `yield` expression used as a keyword value also needs parentheses.

```toml config
[project]
requires-python = ">=3.14"
```

```python report
import codecs


def read(path):
    with codecs.open(path, "r", "utf-8", (yield)) as handle:
        return handle.read()


def encoding_name():
    return codecs.lookup("utf-8").name
```

```python fixed
import codecs


def read(path):
    with open(path, "r", encoding="utf-8", errors=(yield), newline="") as handle:
        return handle.read()


def encoding_name():
    return codecs.lookup("utf-8").name
```

## Other tools

- Ruff's preview rule [PLW1514](https://docs.astral.sh/ruff/rules/unspecified-encoding/) reports `codecs.open()` calls
  that omit the encoding.
