Flags a trailing call in a runnable module when importing that module would execute the call.

## Why is this bad?

Python executes top-level statements when it imports a module. A command can run during import if its final statement
calls its entrypoint without an `if __name__ == "__main__":` guard. A
[console script](https://packaging.python.org/en/latest/specifications/entry-points/#use-for-scripts) imports the
configured callable before calling it, so an unguarded call can run the command twice.

## Example

The unsafe fix wraps the call:

```python report file=command.py
#!/usr/bin/env python


def launch() -> None:
    print("ready")


launch()
```

```python fixed file=command.py
#!/usr/bin/env python


def launch() -> None:
    print("ready")


if __name__ == "__main__":
    launch()
```

## What it does

The rule checks the final top-level statement of a module. A direct call, a call inside
[`sys.exit(...)`](https://docs.python.org/3/library/sys.html#sys.exit), and a call inside `raise SystemExit(...)` count.
The callable's name does not matter.

### Active on

A call is reported when one of these signals identifies a runnable module:

- **`__main__.py`**: the file has this name. Python's
  [`__main__.py` guidance](https://docs.python.org/3/library/__main__.html#idiomatic-usage) commonly uses a bare call
  there. This rule requires the guard because a package's `__main__.py` can also be imported as a module.

  ```python report file=pkg/__main__.py
  def main() -> None:
      print("ready")


  main()
  ```

  ```python fixed file=pkg/__main__.py
  def main() -> None:
      print("ready")


  if __name__ == "__main__":
      main()
  ```

- **Shebang**: the file begins with `#!`.

  ```python report file=tasks.py
  #!/usr/bin/env python3


  def sync() -> None:
      print("synced")


  sync()
  ```

  ```python fixed file=tasks.py
  #!/usr/bin/env python3


  def sync() -> None:
      print("synced")


  if __name__ == "__main__":
      sync()
  ```

- **Executable bit**: the file has an executable bit on Unix.

  ```python report file=tasks.py executable
  def sync() -> None:
      print("synced")


  sync()
  ```

  ```python fixed file=tasks.py
  def sync() -> None:
      print("synced")


  if __name__ == "__main__":
      sync()
  ```

- **Script target**: `[project.scripts]` or `[project.gui-scripts]` names the called object, such as `main` in
  `pkg = "pkg.cli:main"`.

  ```toml config
  [project]
  name = "pkg"
  version = "1"

  [project.scripts]
  pkg = "pkg.cli:main"
  ```

  ```python context file=pkg/__init__.py
  """The package."""
  ```

  ```python report file=pkg/cli.py
  def main() -> None:
      print("ready")


  main()
  ```

  ```python fixed file=pkg/cli.py
  def main() -> None:
      print("ready")


  if __name__ == "__main__":
      main()
  ```

### Exceptions

- **Ordinary module**: a module with no signal above is left alone, even if its final call is named `main()` or `run()`.

  ```python ok
  def run() -> None:
      print("syncing")


  run()
  ```

- **Assignments**: such as `CACHE = populate_cache()` are not entrypoint calls.

  ```python ok file=pkg/__main__.py
  def populate_cache() -> dict[str, int]:
      return {}


  CACHE = populate_cache()
  ```

- **Calls under a main guard**: a call under `if __name__ == "__main__":` is allowed.

  ```python ok file=pkg/__main__.py
  def launch() -> None:
      print("ready")


  if __name__ == "__main__":
      launch()
  ```

- **Unrelated trailing call in a script module**: when only `[project.scripts]` or `[project.gui-scripts]` identify the
  module, an unrelated trailing call, such as `populate_cache()`, is left alone.

  ```toml config
  [project]
  name = "pkg"
  version = "1"

  [project.scripts]
  pkg = "pkg.cli:main"
  ```

  ```python context file=pkg/__init__.py
  """The package."""
  ```

  ```python ok file=pkg/cli.py
  def main() -> None:
      print("ready")


  def populate_cache() -> None:
      print("cache")


  populate_cache()
  ```

### Fix

The unsafe fix wraps a single-line statement in a main guard. It preserves an inline comment and the file's line
endings. Moving the statement changes import-time behavior, so the fix requires `--fix --unsafe-fixes`. Multiline
statements and statements sharing a line with other code remain reports without a fix.
