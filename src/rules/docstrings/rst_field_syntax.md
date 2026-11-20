Flags malformed and misplaced fields in reStructuredText docstrings, such as a misspelled field name or a Google-style
section.

## Why is this bad?

A misspelling such as `:parm path:` prevents
[Sphinx](https://www.sphinx-doc.org/en/master/usage/domains/python.html#info-field-lists) from displaying the entry as a
parameter. A missing blank line after a paragraph can make the field part of the paragraph. Using one spelling for each
field also makes the source searchable.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
def read(path: str) -> str:
    """Read a file.

    :parm path: Where it is.
    """
```

Use instead:

```python ok
def read(path: str) -> str:
    """Read a file.

    :param path: Where it is.
    """
```

## Configuration

- [`docstring-style`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#docstring-markup) in
  `[tool.simplint]`: `"rst"` turns the rule on

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def read(path: str) -> str:
      """Read a file.

      :parm path: Where it is.
      """
      return path
  ```

  ```python ok
  def read(path: str) -> str:
      """Read a file.

      :parm path: Where it is.
      """
      return path
  ```

## What it does

With `docstring-style = "rst"`, simplint checks field lists in function, method and class docstrings. It can rename an
alias and add a missing blank line. Other corrections depend on what the author meant.

### Active on

- **Unknown field names**: a name Sphinx does not know, such as `:parm path:`. The known names are in the table below.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def read(path: str) -> str:
      """Read a file.

      :parm path: Where it is.
      """
      return path
  ```

- **Fields missing what they describe**: `:param:` without a name or `:raises:` without an exception, and a `:returns:`,
  `:rtype:`, `:yields:` or `:ytype:` field with a name.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def read(path: str) -> str:
      """Read a file.

      :param: Where it is.
      """
      return path
  ```

- **Repeated fields**: a field that repeats an earlier one, such as two `:param path:` fields.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def read(path: str) -> str:
      """Read a file.

      :param path: Where it is.
      :param path: Where it is again.
      """
      return path
  ```

- **Aliases of field names**: `:arg path:` and the other spellings in the table below. **Unsafe fix:** the fix renames
  the field to its name in the table and keeps the rest of the line, and code may read the docstring through `__doc__`.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def read(path: str) -> str:
      """Read a file.

      :arg path: Where it is.
      """
      return path
  ```

  ```python fixed
  def read(path: str) -> str:
      """Read a file.

      :param path: Where it is.
      """
      return path
  ```

- **Fields after prose**: a field that follows prose which came after an earlier field, reported once per docstring.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def read(path: str) -> str:
      """Read a file.

      :param path: Where it is.

      Open it in text mode.

      :returns: The text.
      """
      return path
  ```

- **Field lists right under a line of text**: without the blank line, reStructuredText reads the field list as the end
  of that paragraph rather than as a field list. **Unsafe fix:** the fix adds the blank line, and code may read the
  docstring through `__doc__`.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def read(path: str) -> str:
      """Read a file.
      :param path: Where it is.
      """
      return path
  ```

  ```python fixed
  def read(path: str) -> str:
      """Read a file.

      :param path: Where it is.
      """
      return path
  ```

- **Sections of other docstring styles**: a Google-style or numpydoc section in an rst docstring is reported.

  - A Google-style section, such as `Args:`

    ```toml config
    [tool.simplint]
    docstring-style = "rst"
    ```

    ```python report
    def read(path: str) -> str:
        """Read a file.

        Args:
            path: Where it is.
        """
        return path
    ```

  - A numpydoc section, such as `Parameters` over a line of dashes

    ```toml config
    [tool.simplint]
    docstring-style = "rst"
    ```

    ```python report
    def read(path: str) -> str:
        """Read a file.

        Parameters
        ----------
        path : str
            Where it is.
        """
        return path
    ```

### Exceptions

- **Repeated `:raises:` and `:meta:` fields.** Two `:raises ValueError:` fields can describe different failures. The
  rule also allows repeated metadata fields.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def read(path: str) -> str:
      """Read a file.

      :raises ValueError: When the path is empty.
      :raises ValueError: When the path is not UTF-8.
      """
      return path
  ```

- **A role opening a sentence, such as `` :class:`Path` ``**: a backtick, not a space, follows its `:name:`, so it is
  not a field.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def read(path: str) -> str:
      """:class:`Path` is read here.

      :param path: Where it is.
      """
      return path
  ```

- **The lines of a literal block**: they sit deeper than the docstring's own indentation, so they are not fields.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def read(path: str) -> str:
      """Read a file.

      Example::

          :parm path: Not a field, only text.
      """
      return path
  ```

### What counts as a field

A field is a line at the docstring's own indentation that starts with `:name:` followed by a space or the end of the
line.

simplint recognizes the following fields and reports the aliases in the second column.

| Field     | Reported aliases                                                     |
| --------- | -------------------------------------------------------------------- |
| `param`   | `parameter`, `arg`, `argument`, `key`, `keyword`, `kwarg`, `kwparam` |
| `type`    | `paramtype`                                                          |
| `raises`  | `raise`, `except`, `exception`                                       |
| `var`     |                                                                      |
| `ivar`    |                                                                      |
| `cvar`    |                                                                      |
| `vartype` |                                                                      |
| `returns` | `return`                                                             |
| `rtype`   |                                                                      |
| `meta`    |                                                                      |
| `yields`  | `yield`                                                              |
| `ytype`   |                                                                      |

Sphinx groups `var`, `ivar` and `cvar` under Variables. simplint preserves the distinction between instance attributes
and class variables for [rst-attribute-kind](SMP922.html). The generator fields `yields`, `yield` and `ytype` are
additions to the
[Python domain's info fields](https://www.sphinx-doc.org/en/master/usage/domains/python.html#info-field-lists).

Field fixes require a single triple-quoted string whose text matches the source. The field must start below the opening
quotes.

## Other tools

- [Ruff issue #458](https://github.com/astral-sh/ruff/issues/458) tracks darglint checks, including DAR001 for
  docstrings that fail to parse. [Ruff PR #13286](https://github.com/astral-sh/ruff/pull/13286) proposes Sphinx-style
  field support for Ruff's pydoclint rules.
