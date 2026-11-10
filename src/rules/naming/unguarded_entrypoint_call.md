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

The rule checks the final top-level statement of a module. A direct call, a call inside `sys.exit(...)`, and a call
inside `raise SystemExit(...)` count. The callable's name does not matter.

It reports a call when one of these signals identifies a runnable module:

- The file is named `__main__.py`.
- The file begins with a shebang or has an executable bit on Unix.
- `[project.scripts]` or `[project.gui-scripts]` names the called object. In this case an unrelated trailing call, such
  as `populate_cache()`, is left alone.

An ordinary module with no such signal is left alone even if its final call is named `main()` or `run()`. Assignments
such as `CACHE = populate_cache()` are not entrypoint calls. Calls under a main guard are allowed.

Python's [`__main__.py` guidance](https://docs.python.org/3/library/__main__.html#idiomatic-usage) commonly uses a bare
call there. This rule requires the guard because a package's `__main__.py` can also be imported as a module.

## Fix

The unsafe fix wraps a single-line statement in a main guard. It preserves an inline comment and the file's line
endings. Moving the statement changes import-time behavior, so the fix requires `--fix --unsafe-fixes`. Multiline
statements and statements sharing a line with other code remain reports without a fix.
