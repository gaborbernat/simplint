Flags `main.py` and `_main.py` when they serve as command entrypoints.

## Why is this bad?

`python -m package` runs the package's
[`__main__.py`](https://docs.python.org/3/library/__main__.html#main-py-in-python-packages). Pointing the console script
at a function in that file gives both commands the same entrypoint. For a larger command, `__main__.py` can import the
implementation from another module.

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

Either of the following identifies `main.py` or `_main.py` as an entrypoint.

### Active on

- **Script target**: a `[project.scripts]` or `[project.gui-scripts]` entry in `pyproject.toml` names its module, as
  `pkg.main` does in `pkg = "pkg.main:run"`.

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

- **Main guard**: the file has a top-level `if __name__ == "__main__":` block.

  ```python report file=main.py
  def run() -> None:
      print("ready")


  if __name__ == "__main__":
      run()
  ```

### Exceptions

- **Implementation modules:** `_main.py` can hold a command's implementation when the configured entrypoint names
  `__main__.py` instead.

  ```toml config
  [project]
  name = "pkg"
  version = "1"

  [project.scripts]
  pkg = "pkg.__main__:run"
  ```

  ```python context file=pkg/__init__.py
  """The package."""
  ```

  ```python context file=pkg/__main__.py
  from pkg._main import run
  ```

  ```python ok file=pkg/_main.py
  def run() -> None:
      print("ready")
  ```

- **Other entry points:** plugin objects in `[project.entry-points]` do not make their module a command entrypoint.

  ```toml config
  [project]
  name = "pkg"
  version = "1"

  [project.entry-points."pkg.plugins"]
  main = "pkg.main:Plugin"
  ```

  ```python ok file=pkg/main.py
  class Plugin:
      pass
  ```

### Fix

No fix is offered. Renaming an entrypoint also requires updating its imports and script declaration.
