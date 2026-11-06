Flags malformed and misplaced fields in reStructuredText docstrings, such as a misspelled field name or a Google-style
section. Sphinx renders such a field as plain text or prose, so the reference loses the formatted parameter list.

## Why is this bad?

Sphinx renders a field it does not know as a plain field list, and a section from another style as prose, so the
reference shows a misspelled parameter or a Google-style block in place of the formatted parameter list. A duplicate or
misplaced field shows twice or apart from the others. A field spelled with one of Sphinx's aliases, such as `:arg:` for
`:param:`, renders the same, yet a project that mixes spellings reads unevenly and a search for `:param` misses fields.

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

Checks the field lists of reStructuredText docstrings, as
[Sphinx's Python domain](https://www.sphinx-doc.org/en/master/usage/domains/python.html#info-field-lists) reads them,
when `docstring-style = "rst"`. No fix is offered for most reports, since the intended field depends on what the author
meant.

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

- **Aliases of field names**: `:arg path:` and the other spellings in the table below. **Fix:** the fix renames the
  field to its name in the table and keeps the rest of the line.

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

- **Fields after prose**: a field after prose that follows the field list, reported once per docstring.

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

- **Two `:raises ValueError:` fields**: they may document two conditions that raise the same exception, so they are not
  repeated fields.

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

The known field names, with the aliases Sphinx also reads and the rule reports:

| Field     | Reported aliases                                 |
| --------- | ------------------------------------------------ |
| `param`   | `parameter`, `arg`, `argument`, `key`, `keyword` |
| `type`    |                                                  |
| `raises`  | `raise`, `except`, `exception`                   |
| `var`     |                                                  |
| `ivar`    |                                                  |
| `cvar`    |                                                  |
| `vartype` |                                                  |
| `returns` | `return`                                         |
| `rtype`   |                                                  |
| `meta`    |                                                  |
| `yields`  | `yield`                                          |
| `ytype`   |                                                  |

Sphinx renders `var`, `ivar` and `cvar` alike, yet they say different things: `ivar` documents an instance attribute and
`cvar` a class variable, which [rst-attribute-kind](SMP922.html) checks. So none of them counts as an alias of another.

`yields`, its alias `yield` and `ytype` are not Sphinx's own: documentation tools accept them for generators.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open pull request
  [#13286](https://github.com/astral-sh/ruff/pull/13286), for issue
  [#458](https://github.com/astral-sh/ruff/issues/458).
