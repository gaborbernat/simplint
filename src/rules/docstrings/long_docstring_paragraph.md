Flags docstring paragraphs longer than 100 words by default.

## Why is this bad?

Separate the instructions for calling a function from details such as its caching policy so readers can find either in
hover help or the rendered reference. The default limit of 100 words is shared with [long-paragraph](SMP002.html).

## Example

```python report
def load(path: str) -> dict[str, str]:
    """Load the manifest at a path.

    Manifests are cached by path and modification time, because parsing them dominates the start-up time of large
    projects. An entry written by one interpreter is not valid for another, since the parsed form depends on the version
    of the grammar, so the cache directory is named after the interpreter. Entries older than thirty days are removed on
    the next run, and a corrupt entry is discarded rather than reported, because the file it came from is still on disk.
    Writes go to a temporary file first, then replace the entry, so a crash never leaves half of one behind. Readers
    never lock, since a replaced file is either the old one or the new one.
    """
    return {}
```

Use instead:

```python ok
def load(path: str) -> dict[str, str]:
    """Load the manifest at a path.

    Manifests are cached by path and modification time, because parsing them dominates the start-up time of large
    projects. An entry written by one interpreter is not valid for another, so the cache directory is named after it.

    Entries older than thirty days are removed on the next run, and a corrupt entry is discarded rather than reported,
    because the file it came from is still on disk. Writes go to a temporary file first, then replace the entry, so a
    crash never leaves half of one behind.
    """
    return {}
```

## Configuration

- [`max-paragraph-words`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#max-paragraph-words)
  in `[tool.simplint]`: the most words a paragraph may hold, 100 by default, shared with [long-paragraph](SMP002.html)

  ```toml config
  [tool.simplint]
  max-paragraph-words = 10
  ```

  ```python report
  def load() -> None:
      """The cache keeps one entry per manifest, keyed by path and modification time."""
  ```

## What it does

No fix is offered, since where to split a paragraph depends on its meaning.

### Active on

- **Docstring paragraphs**: a paragraph is a run of non-blank lines, and it is reported once it passes
  `max-paragraph-words` words. A blank line ends it. Docstrings of modules, classes and functions, methods included, are
  checked.

  ```python report
  """The cache keeps one entry per manifest, keyed by path and modification time, because parsing a manifest dominates
  the start-up time of large projects. An entry written by one interpreter is not valid for another, since the parsed
  form depends on the version of the grammar, so the cache directory is named after the interpreter. Entries older than
  thirty days are removed on the next run, and a corrupt entry is discarded rather than reported, because the file it
  came from is still on disk and can be parsed again. Readers never lock, since a replaced file is either the old one or
  the new one."""
  ```

  ```python ok
  """The cache keeps one entry per manifest, keyed by path and modification time, because parsing a manifest dominates
  the start-up time of large projects. An entry written by one interpreter is not valid for another.

  Entries older than thirty days are removed on the next run, and a corrupt entry is discarded rather than reported,
  because the file it came from is still on disk and can be parsed again."""
  ```

### Exceptions

- **Doctest lines**: a line starting with `>>>` or `...` is code, not prose, so it ends the paragraph before it and
  counts toward none.

  ```toml config
  [tool.simplint]
  max-paragraph-words = 10
  ```

  ```python ok
  def double(value: int) -> int:
      """Double a value.

      >>> double(2) + double(3) + double(4) + double(5) + double(6) + double(7)
      54
      """
      return value * 2
  ```

- **Structured blocks**: a list item, an RST field such as `:param path:`, a directive, a Google section header or
  entry, and a numpydoc entry each start a new paragraph, and a heading underline ends one, so each item counts on its
  own.

  ```toml config
  [tool.simplint]
  max-paragraph-words = 10
  ```

  ```python ok
  def connect(host: str, port: int) -> None:
      """Open a connection.

      Args:
          host: Name or address the client connects to.
          port: TCP port the server listens on.
      """
  ```

### Report range

The report starts on the paragraph's first line and ends on the line that exceeds the limit. The rule counts each part
of an implicitly concatenated docstring separately.
