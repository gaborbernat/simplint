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
explains it better.

## Example

```python
def load_config(path: Path) -> Config:
    """Load the config from the given path."""
    return Config.from_toml(path.read_text())
```

Use instead:

```python
def load_config(path: Path) -> Config:
    return Config.from_toml(path.read_text())
```

## Fix safety

The fix deletes the docstring and the blank lines after it. When the docstring is the whole body, the fix replaces it
with [`...`](https://docs.python.org/3/builtins/constants.html#Ellipsis), dropping a pragma comment after it such as
`# noqa: E501`, which covered the docstring's line only. The fix is always unsafe, because code that reads `__doc__`
sees the change, such as [doctest](https://docs.python.org/3/library/doctest.html) or
[`help()`](https://docs.python.org/3/builtins/functions.html#help). A docstring that shares its line with the next
statement goes up to that statement; no fix is offered when a comment sits between them.

## Conflicts with Ruff

Ruff's [undocumented-public-class (D101)](https://docs.astral.sh/ruff/rules/undocumented-public-class/),
[undocumented-public-method (D102)](https://docs.astral.sh/ruff/rules/undocumented-public-method/) and
[undocumented-public-function (D103)](https://docs.astral.sh/ruff/rules/undocumented-public-function/) ask for the
docstrings this rule removes. While they are enabled, simplint withdraws the fix and keeps the report; turn them off and
let [SMP116](SMP116.html) ask for documentation on the public API.
