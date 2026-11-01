## What it does

Checks for [docstrings](https://docs.python.org/3/glossary.html#term-docstring) on internal functions, methods and
classes, at any nesting depth. The module's public API is exempt, since [SMP116](SMP116.html) requires it to be
documented: every function and class listed in
[`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package) or imported by another module in
the run, and the methods of an exported class whose names do not start with `_`. Its dunder methods, such as `__init__`,
its overrides and its property setters keep their docstrings too, since documentation tools show them. When the module
builds `__all__` dynamically, every name without a leading `_` is treated as possibly public and skipped. Module
docstrings are left to [SMP003](SMP003.html) and [SMP004](SMP004.html).

Use SMP116 in place of Ruff's [pydocstyle rules](https://docs.astral.sh/ruff/rules/#pydocstyle-d) that ask for
docstrings, such as
[undocumented-public-function (D103)](https://docs.astral.sh/ruff/rules/undocumented-public-function/) and
[undocumented-public-class (D101)](https://docs.astral.sh/ruff/rules/undocumented-public-class/): they treat every name
without a leading `_` as public, while SMP116 follows `__all__`.

## Why is this bad?

A docstring restates what the name, signature and type annotations already say, and nothing checks that it stays
accurate when the code changes. When the reason behind a piece of code is not obvious, a short comment next to that code
explains it better. A longer docstring may state a contract the code does not show, such as a tie-break rule, so the fix
keeps its text as a comment.

## Example

```python
def load_config(path: Path) -> Config:
    """Load the config file."""
    return Config.from_toml(path.read_text())
```

Use instead:

```python
def load_config(path: Path) -> Config:
    return Config.from_toml(path.read_text())
```

A docstring that says more becomes a comment:

```python
def _pick(items: list[int]) -> int:
    """Return the largest item, or the first one on a tie."""
    return max(items)
```

Becomes:

```python
def _pick(items: list[int]) -> int:
    # Return the largest item, or the first one on a tie.
    return max(items)
```

## Fix safety

A docstring of one line with at most five words, such as `"""Return the value."""` or `"""Helper class."""`, restates
the name, and the fix deletes it with the blank lines after it. Any longer docstring may state a contract, and the fix
turns it into a `#` comment block in its place:

- the text is the docstring's source between its quotes, escapes such as `\n` as written, cleaned as
  [`inspect.cleandoc`](https://docs.python.org/3/library/inspect.html#inspect.cleandoc) gives it: without the quotes,
  the common indentation, and the blank lines around it, with each line's trailing whitespace dropped
- blank lines between paragraphs become a lone `#`, and lines made only of heading underline characters, such as the
  `----------` under a numpydoc `Parameters`, are dropped, since as comments they read as section dividers (see
  [SMP001](SMP001.html))
- lines that fit keep their text and relative indentation, so lists and indented blocks keep their shape; a longer line
  wraps at spaces, and what wraps over joins the next line when that line goes on with the same sentence: it has the
  same indentation, starts with a lowercase letter, and does not open with a field such as `name:` or `name (int):`. A
  URL longer than the room gets a line of its own
- the room is the shorter of Ruff's line length limit and its
  [`max-doc-length`](https://docs.astral.sh/ruff/settings/#lint_pycodestyle_max-doc-length), at the indentation
  `ruff format` gives the body, so the result does not depend on how the code is laid out now

When the docstring is the whole body, either fix leaves
[`...`](https://docs.python.org/3/builtins/constants.html#Ellipsis) as the body, after the comment when there is one. A
pragma comment after the docstring, such as `# noqa: E501`, covered the docstring's line only and goes with it; another
comment stays, after the `...` or at the end of the last comment line. Like the deletion, the comment takes the blank
lines after the docstring. A comment cannot share a line with code before it, so the comment for a docstring on the
`def` line, or sharing its line with the next statement, goes on lines of its own, and so do the statements after it, as
`ruff format` would put them.

No fix is offered when:

- a comment sits between the docstring and a statement sharing its line;
- a comment line would read as a tool directive, such as one starting with `type:`, `noqa`, `fmt:` or `simplint:`, or
  holding `# noqa` in its text;
- a word too long for the room, other than a URL ending the line, would leave a comment line Ruff's
  [line-too-long (E501)](https://docs.astral.sh/ruff/rules/line-too-long/) reports, which it skips on a docstring line
  of a single word;
- `ruff format` leaves the docstring alone, under `# fmt: off`, with `# fmt: skip` or in an excluded file, and its
  indentation is not a multiple of the indent width, which Ruff's
  [indentation-with-invalid-multiple-comment (E114)](https://docs.astral.sh/ruff/rules/indentation-with-invalid-multiple-comment/)
  reports for comments.

The fix is always unsafe, because code that reads `__doc__` sees the change, such as
[doctest](https://docs.python.org/3/library/doctest.html) or
[`help()`](https://docs.python.org/3/builtins/functions.html#help).

## Conflicts with Ruff

Ruff's [undocumented-public-class (D101)](https://docs.astral.sh/ruff/rules/undocumented-public-class/),
[undocumented-public-method (D102)](https://docs.astral.sh/ruff/rules/undocumented-public-method/),
[undocumented-public-function (D103)](https://docs.astral.sh/ruff/rules/undocumented-public-function/),
[undocumented-magic-method (D105)](https://docs.astral.sh/ruff/rules/undocumented-magic-method/),
[undocumented-public-nested-class (D106)](https://docs.astral.sh/ruff/rules/undocumented-public-nested-class/) and
[undocumented-public-init (D107)](https://docs.astral.sh/ruff/rules/undocumented-public-init/) ask for the docstrings
this rule removes. simplint stops with an error before linting while any of them applies to a file this rule checks;
turn them off and let [SMP116](SMP116.html) ask for documentation on the public API, or turn this rule off.

When the comment the fix writes gives a Ruff rule a new report, such as
[commented-out-code (ERA001)](https://docs.astral.sh/ruff/rules/commented-out-code/) for code in the docstring, simplint
withdraws the fix and keeps the report.
