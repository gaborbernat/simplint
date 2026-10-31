## What it does

Checks for `try` statements whose body holds only imports of required modules and that have an `except` handler catching
[`ImportError`](https://docs.python.org/3/builtins/exceptions.html#ImportError) or
[`ModuleNotFoundError`](https://docs.python.org/3/builtins/exceptions.html#ModuleNotFoundError), alone or in a tuple,
also when spelled `builtins.ImportError` or `builtins.ModuleNotFoundError`. A module is required when its top-level
package:

- is in the standard library of every Python version from the target onward, set by Ruff's
  [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) or
  [`requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python), so a
  module removed later, such as `distutils` in 3.12, does not count. Modules that exist on one platform only, such as
  `fcntl`, `msvcrt` or `winreg`, or that minimal and alternative builds may leave out, such as `ctypes`, `ssl`,
  `sqlite3` or `tkinter`, never count
- is listed in
  [`project.dependencies`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#dependencies) of
  `pyproject.toml`

[Relative imports](https://docs.python.org/3/reference/import.html#package-relative-imports) and imports of the package
the file belongs to never count as required, so guarding an optional compiled extension of your own package is not
reported.

A requirement with an [environment marker](https://peps.python.org/pep-0508/#environment-markers) (`;`) is conditional
and does not count. Optional dependencies and dependency groups are not read. Dependencies are matched by their
[normalized name](https://packaging.python.org/en/latest/specifications/name-normalization/). A short list of well-known
distributions that import under another name also match that name, such as `PyYAML` as `yaml`, `Pillow` as `PIL` or
`beautifulsoup4` as `bs4`. Other renamed distributions are not recognized.

## Why is this bad?

A required dependency is always installed, so the fallback never runs in a working environment. When the install is
broken, the guard hides the import error until something later fails with a less clear message.

## Example

```python
try:
    import requests
except ImportError:
    requests = None
```

Use instead:

```python
import requests
```

## Fix safety

The fix replaces the `try` statement with its dedented body and drops every `except` handler. It is always unsafe,
because the fallback code is removed and a failing import now raises. No fix is offered when the statement has an `else`
or `finally` clause, or shares a line with other code.

## Options

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python)
- [`project.dependencies`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#dependencies)
