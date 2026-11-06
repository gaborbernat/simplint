Flags Sphinx roles whose target lacks backticks and inline literals left unclosed in reStructuredText docstrings. Such
markup renders as plain text, or turns the rest of the paragraph into code.

## Why is this bad?

Without backticks a role is plain text: the reference shows `:class:Path` instead of a link. An unclosed inline literal
turns the rest of the paragraph into code, or makes Sphinx warn and show the markup as written.

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

### Exceptions

- **Literal blocks indented under a paragraph ending in `::`**: their content is code, which holds no inline markup.

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

- [flake8-rst-docstrings](https://github.com/peterjc/flake8-rst-docstrings) reports an inline literal with no closing
  backticks as [RST214](https://github.com/peterjc/flake8-rst-docstrings), with no fix.
