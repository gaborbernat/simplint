Flags docstring paragraphs longer than 100 words by default, since a paragraph that long usually mixes topics and gets
skimmed.

## Why is this bad?

A docstring is read in the rendered reference and in an editor's hover help, where a long block of text is skimmed. A
paragraph that long usually covers more than one topic; split at a change of topic, each part is easier to find and to
keep accurate. [long-paragraph](SMP002.html) asks the same of comments.

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

### Report range

The report spans from the paragraph's first line to the line where it passes `max-paragraph-words` words.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
