Flags `codecs.open()` calls, which the builtin `open()` replaces.

## Why is this bad?

Python 3.14 deprecates `codecs.open()`, and the builtin `open()` takes the same arguments. With an encoding,
`codecs.open()` opens the file in binary mode and decodes it through a stream wrapper. That wrapper splits lines at the
characters `str.splitlines()` honors, such as form feeds and U+2028, where `open()` splits only at line endings. With
`newline=""`, `open()` also keeps the line endings as they are, as `codecs.open()` does.

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

No setting changes the report. The rule reports on every target version, since `open()` takes the arguments in each of
them.

## What it does

### Active on

- **Calls of `codecs.open`**, by its qualified name or by a name imported from `codecs`. **Unsafe fix:** the call
  becomes `open(...)` with `encoding=` and `newline=""`. Reading and writing whole files give the same text and bytes,
  but iterating over lines does not for text with a form feed, U+2028, U+001C or U+0085: `codecs.open()` yields a line
  at each of them, and `open()` does not.

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

No fix is offered when:

- no encoding is given, or it is `None`: `codecs.open()` then returns the builtin text file and ignores `errors`;

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import codecs


  def read_text(path):
      return codecs.open(path).read()
  ```

- the mode is not a string literal, or it has a `b`: a binary mode with an encoding returns a stream that `open()`
  rejects;

  ```toml config
  [project]
  requires-python = ">=3.10"
  ```

  ```python report
  import codecs


  def read_bytes(path):
      return codecs.open(path, "rb", "utf-8").read()
  ```

- a Ruff rule the project enables still flags the new `open()` call after Ruff's own fixes, such as PTH123
  (`builtin-open`) under the default settings, so simplint withdraws the fix;

- an argument is unpacked with `*` or `**`, or `open` is bound in the module, or by a parameter, local or loop variable
  of a function, lambda, class body or comprehension that encloses the call, which the new call would pick up;

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

- the call has a comment inside it, which the rewrite would drop.

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

A value passed as `encoding`, `errors` or `buffering` that needs parentheses, such as a walrus or a `yield`, keeps the
parentheses it has in the source and gets them when it has none, since `encoding=name := value` is a syntax error:

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

A `yield` passed as `errors` is parenthesized in the keyword the same way:

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

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet. Preview rule
  [PLW1514](https://docs.astral.sh/ruff/rules/unspecified-encoding/) reports a `codecs.open()` without an encoding,
  asking for one rather than for `open()`.
