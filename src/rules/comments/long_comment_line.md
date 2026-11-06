Flags comment and docstring lines longer than the comment width, the stricter of Ruff's `line-length` and
`pycodestyle.max-doc-length`, when it can reflow the paragraph they belong to.

## Why is this bad?

A line past the width scrolls sideways in an editor and breaks the column the rest of the file keeps. Ruff's
line-too-long rule reports these lines but does not wrap them, and `ruff format` leaves comments and docstrings alone,
so they stay long until someone wraps them by hand.

## Example

```toml config
[tool.ruff]
line-length = 60
```

```python report
# Entries older than thirty days are removed on the next run, and a corrupt entry is discarded.
VALUE = 1
```

Use instead:

```toml config
[tool.ruff]
line-length = 60
```

```python ok
# Entries older than thirty days are removed on the next
# run, and a corrupt entry is discarded.
VALUE = 1
```

## What it does

### Active on

- **Paragraphs of comments**: consecutive own-line comments at the same indentation, whose text is prose, form one
  paragraph. A paragraph with a line past the width is reported once, at that line, and its fix rewraps the whole
  paragraph at the width.

  ```toml config
  [tool.ruff]
  line-length = 60
  ```

  ```python report
  # Entries older than thirty days are removed on the next run, and a corrupt entry is discarded.
  VALUE = 1
  ```

  ```python fixed
  # Entries older than thirty days are removed on the next
  # run, and a corrupt entry is discarded.
  VALUE = 1
  ```

- **Paragraphs of docstrings**: the lines of a docstring form paragraphs the same way, split at blank lines. The first
  line of a docstring starts after its opening quotes, so it gets the room that is left after them.

  ```toml config
  [tool.ruff]
  line-length = 60
  ```

  ```python report
  def fetch() -> None:
      """Fetch the manifest of the project, and keep it until the next run, which removes old entries."""
  ```

  ```python fixed
  def fetch() -> None:
      """Fetch the manifest of the project, and keep it until
      the next run, which removes old entries."""
  ```

- **Lines broken on purpose**: a line that had room for the first word of the line below it ends its paragraph, as a
  label above a sentence does, so the fix keeps it on its own.

  ```toml config
  [tool.ruff]
  line-length = 60
  ```

  ```python report
  # Cleanup
  # Entries older than thirty days are removed on the next run, and a corrupt entry is discarded.
  VALUE = 1
  ```

  ```python fixed
  # Cleanup
  # Entries older than thirty days are removed on the next
  # run, and a corrupt entry is discarded.
  VALUE = 1
  ```

### Exceptions

- **Lines it cannot reflow**: a list item, a doctest, a field list, a heading underline, a table whose columns are
  aligned with runs of spaces, or a line indented past its paragraph stands alone, since rewrapping it could change its
  meaning. Ruff's line-too-long rules report these lines, so this rule leaves them to Ruff, as it does a paragraph
  holding a word wider than the room on a line or a single-quoted docstring.

  ```toml config
  [tool.ruff]
  line-length = 60
  ```

  ```python ok
  # - Entries older than thirty days are removed on the next run, and a corrupt entry is discarded.
  VALUE = 1
  ```

  ```toml config
  [tool.ruff]
  line-length = 60
  ```

  ```python ok
  # name                 kind               default    since version
  VALUE = 1
  ```

- **Trailing comments**: a comment after code shares the line with the code, which Ruff's line length governs.

  ```toml config
  [tool.ruff]
  line-length = 60
  ```

  ```python ok
  VALUE = 1  # Entries older than thirty days are removed on the next run, and a corrupt entry is discarded.
  ```

- **Directives**: `# noqa`, `# type: ignore`, `# fmt: off` and the other directives of tools stay as they are, since
  rewording them changes what the tool does.

  ```toml config
  [tool.ruff]
  line-length = 60
  ```

  ```python ok
  # Suppressed because the upstream name is fixed by the protocol it implements: # noqa: E501
  VALUE = 1
  ```

- **A word or a URL that ends the line**: a single word, or a line ending in a URL that starts within the width, is left
  alone, as Ruff's line-too-long rule leaves it.

  ```toml config
  [tool.ruff]
  line-length = 60
  ```

  ```python ok
  # https://docs.python.org/3/library/pathlib.html#pathlib.PurePath.relative_to
  VALUE = 1
  ```

- **Shebang lines**: `#!` lines name the interpreter, which the file needs in that form.

### Fix

The fix changes only whitespace, so comment text keeps its meaning and it is safe. A docstring's words are its value,
which the code reads as `__doc__`, so its fix is unsafe.

Lines are measured where `ruff format` puts them, so a run after the formatter finds nothing new: at the formatter's
indentation, a comment inside brackets one level deeper for each bracket, the lines of a docstring shifted with its
least indented line, and the closing quotes of a docstring with one line of text after that text.

## Ruff overlap

Ruff's line-too-long (`E501`) and doc-line-too-long (`W505`) rules report every line this rule reports, and more. Ruff
does not wrap them, so this rule reports only the paragraphs it can reflow, for the sake of its fix. With `E501` or
`W505` enabled, those lines are reported twice: once by Ruff, and once by this rule with the fix. The lines it cannot
reflow are left to Ruff alone.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** released rules [E501](https://docs.astral.sh/ruff/rules/line-too-long/)
  and [W505](https://docs.astral.sh/ruff/rules/doc-line-too-long/) report the same lines without a fix; this rule exists
  for its rewrap fix, which open issue [#7414](https://github.com/astral-sh/ruff/issues/7414) asks Ruff for.
- [pycodestyle](https://github.com/PyCQA/pycodestyle) reports the same lines as
  [W505](https://pycodestyle.pycqa.org/en/latest/intro.html). The check is off by default and needs `max-doc-length`
  set, and it does not fix the line.
