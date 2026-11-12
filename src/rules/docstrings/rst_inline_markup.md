Flags Sphinx roles whose target lacks backticks and inline literals left unclosed in reStructuredText docstrings.

## Why is this bad?

reStructuredText reads a role only on
[text in backquotes](https://docutils.sourceforge.io/docs/ref/rst/restructuredtext.html#interpreted-text). Without them
the role is plain text: the reference shows `:class:Path` instead of a link. An inline literal needs its closing
backticks in the same paragraph. If they are missing, docutils warns that the start-string has no end-string and shows
the backticks as written, with the rest of the paragraph as plain text. A build run with Sphinx's
[`-W`](https://www.sphinx-doc.org/en/master/man/sphinx-build.html#cmdoption-sphinx-build-W) fails on that warning.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
def open_file(path: str) -> None:
    """Open the file at :class:Path, as ``open does."""
```

Use instead:

```python ok
def open_file(path: str) -> None:
    """Open the file at :class:`Path`, as ``open`` does."""
```

## Configuration

- [`docstring-style`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#docstring-markup) in
  `[tool.simplint]`: `"rst"` turns the rule on

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def open_file(path: str) -> None:
      """Open the file at :class:Path."""
  ```

  ```python ok
  def open_file(path: str) -> None:
      """Open the file at :class:Path."""
  ```

## What it does

Checks reStructuredText docstrings, when `docstring-style = "rst"`, for inline markup that does not render as meant.

### Active on

- **Roles without backticks**: a
  [Sphinx role](https://www.sphinx-doc.org/en/master/usage/domains/python.html#cross-referencing-python-objects), such
  as `:class:`, `:func:`, `:meth:`, `:attr:`, `:mod:`, `:exc:` or `:ref:`, whose target lacks backticks, as
  `:class:Path`. A role's target runs to the next whitespace, without the punctuation that ends a sentence. The fix puts
  the target in backticks. **Unsafe fix:** code may read the docstring through `__doc__`. No fix is offered when the
  target holds a character a reference cannot, as the `=` of `:attr:debug=True`, since where the target ends is then for
  the author to say.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def open_file(path: str) -> None:
      """Open the file at :class:Path."""
  ```

  ```python fixed
  def open_file(path: str) -> None:
      """Open the file at :class:`Path`."""
  ```

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def open_file(path: str) -> None:
      """Open the file at :attr:debug=True."""
  ```

- **Unclosed inline literals**: an inline literal opened with double backticks that no later pair closes within its
  paragraph. No fix is offered, since where it should end is for the author to say.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def open_file(path: str) -> None:
      """Open the file, as ``open does."""
  ```

- **Markdown code fences**: a fence written as in Markdown, with three backticks or tildes, is not reStructuredText, so
  its lines render as prose. Use `::` or `.. code-block::` instead. Only the opening fence is reported. A fence with an
  info string, or a bare fence that a closing fence answers in the same paragraph, counts as one.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report file=fence.py
  def open_file(path: str) -> None:
      """Open the file.

      ~~~python
      open_file("a")
      ~~~
      """
  ```

  ```python ok file=fence_ok.py
  def open_file(path: str) -> None:
      """Open the file.

      Example::

          open_file("a")
      """
  ```

- **A literal followed by a letter**: in ``` ``Path``s ``` the closing backticks do not end a word, so the literal stays
  open. Write the suffix after a backslash, as ``` ``Path``\ s ```.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report file=suffix.py
  def open_file(path: str) -> None:
      """Open the file as ``Path``s do."""
  ```

  ```python ok file=suffix_ok.py
  def open_file(path: str) -> None:
      """Open the file as ``Path`` does."""
  ```

- **Mismatched backticks**: one backtick closed by two, or two closed by one, renders wrong even where docutils reads
  the pair as markup. The pair must match, as ``` `Path`` ``` and ``` ``Path` ``` do not.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report file=double.py
  def open_file(path: str) -> None:
      """Open the file at `Path``, then close it."""
  ```

  ```python report file=single.py
  def open_file(path: str) -> None:
      """Open the file at ``Path`, then close it."""
  ```

  ```python ok file=mismatch_ok.py
  def open_file(path: str) -> None:
      """Open the file at ``Path``, then close it."""
  ```

- **Prose directives, such as `.. note::`**: their content is prose, so it is checked.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def open_file(path: str) -> None:
      """Open the file.

      .. note:: Use :class:Path here.
      """
  ```

- **A summary ending in `::`**: the lines after it are prose, unless they are indented past the docstring's body. Here
  the line sits at the body's indentation, so its unclosed inline literal is reported.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def open_file(path: str) -> None:
      """Open the file::

      Use ``open here.
      """
  ```

### Exceptions

- **Literal blocks indented under a paragraph ending in `::`**: their content is code, which holds no inline markup. The
  first line of a docstring follows its opening quotes, so it is measured against the body's indentation in the lines
  after it, and a summary ending in `::` opens a block only for lines indented past that body.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def open_file(path: str) -> None:
      """Open the file.

      Example::

          x = ``open
      """
  ```

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def open_file(path: str) -> None:
      """Open the file::

          handle = ``open(path)

      Close it when done.
      """
  ```

- **The content of a code directive such as `.. code-block::`**: the directive shows code, which holds no inline markup.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def open_file(path: str) -> None:
      """Open the file.

      .. code-block:: python

          x = ``open
      """
  ```

- **Doctest blocks, from a `>>>` line to the next blank line**: they are code, which holds no inline markup.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def open_file(path: str) -> None:
      """Open the file.

      >>> open_file("a") ``b
      """
  ```

- **Docstrings whose escapes make their text differ from the source**: positions in the source hold only for a docstring
  whose text is its source, so the rule does not check them and offers no fix.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def open_file(path: str) -> None:
      """Open the file at \x3aclass:Path."""
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet. Open issue
  [#283](https://github.com/astral-sh/ruff/issues/283) names flake8-rst-docstrings only as a plugin candidate.
- [flake8-rst-docstrings](https://github.com/peterjc/flake8-rst-docstrings) reports an inline literal with no closing
  backticks as [RST214](https://github.com/peterjc/flake8-rst-docstrings), with no fix.
