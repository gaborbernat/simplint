Flags `main.py` and `_main.py` when they serve as command entrypoints.

## Why is this bad?

Python looks for a package's [`__main__.py`](https://docs.python.org/3/library/__main__.html#main-py-in-python-packages)
when it runs `python -m package`. Keeping the command's entrypoint there makes the executable module easy to find and
lets the same package run through `-m`. A separate `_main.py` can still hold the implementation when `__main__.py`
imports and calls it.

## Example

```toml config
[project]
name = "pkg"
version = "1"

[project.scripts]
pkg = "pkg.main:run"
```

```python context file=pkg/__init__.py
"""The package."""
```

```python report file=pkg/main.py
def run() -> None:
    print("ready")
```

Use instead:

```toml config
[project]
name = "pkg"
version = "1"

[project.scripts]
pkg = "pkg.__main__:run"
```

```python ok file=pkg/__main__.py
def run() -> None:
    print("ready")


if __name__ == "__main__":
    run()
```

## What it does

The rule reports a file named `main.py` or `_main.py` when a `[project.scripts]` or `[project.gui-scripts]` entry in
`pyproject.toml` names its module, or when the file has a top-level `if __name__ == "__main__":` block. It does not
report the filename alone.

### Exceptions

- **Implementation modules:** `_main.py` can hold a command's implementation when the configured entrypoint names
  `__main__.py` instead.
- **Other entry points:** plugin objects in `[project.entry-points]` do not make their module a command entrypoint.

## Fix

No fix is offered. Renaming an entrypoint also requires updating its imports and script declaration.
