Flags comment paragraphs longer than 100 words by default, since a paragraph that long usually mixes topics and gets
skimmed.

## Why is this bad?

A paragraph that long usually covers more than one topic, and readers skim it. Split at a change of topic, each part is
easier to find and to keep accurate.

## Example

```python report
# Manifests are cached by path and modification time, because parsing them dominates the start-up time of large
# projects. An entry written by one interpreter is not valid for another, since the parsed form depends on the version
# of the grammar, so the cache directory is named after the interpreter. Entries older than thirty days are removed on
# the next run, and a corrupt entry is discarded rather than reported, because the file it came from is still on disk.
# Writes go to a temporary file first, then replace the entry, so a crash never leaves half of one behind. Readers
# never lock, since a replaced file is either the old one or the new one.
CACHE = {}
```

Use instead:

```python ok
# Manifests are cached by path and modification time, because parsing them dominates the start-up time of large
# projects. An entry written by one interpreter is not valid for another, so the cache directory is named after it.
#
# Entries older than thirty days are removed on the next run, and a corrupt entry is discarded rather than reported,
# because the file it came from is still on disk. Writes go to a temporary file first, then replace the entry, so a
# crash never leaves half of one behind.
CACHE = {}
```

## Configuration

- [`max-paragraph-words`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#max-paragraph-words)
  in `[tool.simplint]`: the most words a paragraph may hold, 100 by default

  ```toml config
  [tool.simplint]
  max-paragraph-words = 10
  ```

  ```python report
  # The cache keeps one entry per manifest, keyed by path and modification time.
  VALUE = 1
  ```

## What it does

No fix is offered, since where to split a paragraph depends on its meaning.

### Active on

- **Own-line comment paragraphs**: a **paragraph** is a run of own-line comments on consecutive lines at the same
  indentation, and it is reported once it passes `max-paragraph-words` words. Any of these ends it:

  - an empty `#` line
  - a line of code
  - a comment at another indentation

  ```python report
  # The cache keeps one entry per manifest, keyed by path and modification time, because parsing a manifest dominates
  # the start-up time of large projects. An entry written by one interpreter is not valid for another, since the parsed
  # form depends on the version of the grammar, so the cache directory is named after the interpreter. Entries older
  # than thirty days are removed on the next run, and a corrupt entry is discarded rather than reported, because the
  # file it came from is still on disk and can be parsed again. Writes go to a temporary file first, then replace the
  # entry, so a crash never leaves half of one behind. Readers never lock, since a replaced file is either the old one
  # or the new one.
  VALUE = 1
  ```

  ```python ok
  # The cache keeps one entry per manifest, keyed by path and modification time, because parsing a manifest dominates
  # the start-up time of large projects. An entry written by one interpreter is not valid for another, since the parsed
  # form depends on the version of the grammar, so the cache directory is named after the interpreter.
  #
  # Entries older than thirty days are removed on the next run, and a corrupt entry is discarded rather than reported,
  # because the file it came from is still on disk and can be parsed again. Writes go to a temporary file first, then
  # replace the entry, so a crash never leaves half of one behind. Readers never lock, since a replaced file is either
  # the old one or the new one.
  VALUE = 1
  ```

### Exceptions

- **Trailing comments**: a comment after code annotates that line, so it is not part of a paragraph.

  ```python ok
  VALUE = 1  # The cache keeps one entry per manifest, keyed by path and modification time, because parsing a manifest dominates the start-up time of large projects. An entry written by one interpreter is not valid for another, since the parsed form depends on the version of the grammar, so the cache directory is named after the interpreter. Entries older than thirty days are removed on the next run, and a corrupt entry is discarded rather than reported, because the file it came from is still on disk and can be parsed again. Writes go to a temporary file first, then replace the entry, so a crash never leaves half of one behind. Readers never lock, since a replaced file is either the old one or the new one.
  ```

### Report range

The report spans from the paragraph's first line to the line where it passes `max-paragraph-words` words.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rule
  [W505](https://docs.astral.sh/ruff/rules/doc-line-too-long/), which limits characters per line rather than words per
  paragraph; merged pull request [#1804](https://github.com/astral-sh/ruff/pull/1804).
