## What it does

Checks the field lists of reStructuredText docstrings, as
[Sphinx's Python domain](https://www.sphinx-doc.org/en/master/usage/domains/python.html#info-field-lists) reads them,
when `docstring-style = "rest"`. It reports:

- a field name Sphinx does not know, such as `:parm path:`; the known names are `param` (or `parameter`, `arg`,
  `argument`, `key`, `keyword`), `type`, `raises` (or `raise`, `except`, `exception`), `var` (or `ivar`, `cvar`),
  `vartype`, `returns` (or `return`), `rtype`, `meta`, and the `yields` (or `yield`) and `ytype` fields documentation
  tools accept for generators
- a field missing what it describes, such as `:param:` without a name or `:raises:` without an exception, and a
  `:returns:`, `:rtype:`, `:yields:` or `:ytype:` field with a name
- a field that repeats an earlier one, such as two `:param path:` fields or two `:raises ValueError:` fields
- a field after prose that follows the field list, once per docstring
- a field list right under a line of text, without the blank line reStructuredText needs to read it as a field list
  rather than as the end of that paragraph
- a `:param:`, `:raises:`, `:returns:`, `:yields:` or `:var:` field with no description, as the fixes of the other
  reStructuredText rules add it
- a Google-style section, such as `Args:`, or a numpydoc one, such as `Parameters` over a line of dashes

A field is a line at the docstring's own indentation that starts with `:name:` followed by a space or the end of the
line, so a role such as `` :class:`Path` `` opening a sentence and the lines of a literal block are not fields.

## Why is this bad?

Sphinx renders a field it does not know as a plain field list, and a section from another style as prose, so the
reference shows a misspelled parameter or a Google-style block in place of the formatted parameter list. A duplicate or
misplaced field shows twice or apart from the others.

## Example

```python
def read(path: str) -> str:
    """Read a file.

    :parm path: Where it is.
    """
```

Use instead:

```python
def read(path: str) -> str:
    """Read a file.

    :param path: Where it is.
    """
```

## Fix safety

For a field list right under a line of text, the fix adds the blank line. It is unsafe, since code may read the
docstring through `__doc__`. No other report has a fix, since the intended field depends on what the author meant.

## Options

- `docstring-style` in `[tool.simplint]`: `"rest"` turns the rule on
