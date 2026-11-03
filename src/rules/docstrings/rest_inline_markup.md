## What it does

Checks reStructuredText docstrings, when `docstring-style = "rest"`, for inline markup that does not render as meant:

- a [Sphinx role](https://www.sphinx-doc.org/en/master/usage/domains/python.html#cross-referencing-python-objects),
  such as `:class:`, `:func:`, `:meth:`, `:attr:`, `:mod:`, `:exc:` or `:ref:`, whose target lacks backticks, as
  `:class:Path`
- an inline literal opened with double backticks that no later pair closes within its paragraph

Lines of a literal block, indented under a line ending in `::`, are code and not checked.

## Why is this bad?

Without backticks a role is plain text: the reference shows `:class:Path` instead of a link. An unclosed inline literal
turns the rest of the paragraph into code, or makes Sphinx warn and show the markup as written.

## Example

```python
def open_file(path: str) -> None:
    """Open the file at :class:Path, as ``open does."""
```

Use instead:

```python
def open_file(path: str) -> None:
    """Open the file at :class:`Path`, as ``open`` does."""
```

## Fix safety

For a role, the fix puts its target in backticks. It is unsafe, since code may read the docstring through `__doc__`.
No fix is offered for an unclosed literal, since where it should end is for the author to say, nor when escapes in the
docstring make its text differ from the source, which the rule then does not check.

## Options

- `docstring-style` in `[tool.simplint]`: `"rest"` turns the rule on
