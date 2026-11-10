Flags a trailing `main()` or `run()` call that runs when its module is imported.

## Why is this bad?

Importing a module executes its top-level statements. A command that calls `main()` or `run()` during import can run
when another module imports its functions or tests inspect it.
[Python's `__main__` documentation](https://docs.python.org/3/library/__main__.html#idiomatic-usage) recommends a guard
for ordinary modules that can run as scripts.

## Example

The unsafe fix wraps the call:

```python report file=command.py
def main() -> None:
    print("ready")


main()
```

```python fixed file=command.py
def main() -> None:
    print("ready")


if __name__ == "__main__":
    main()
```

## What it does

The rule checks the final top-level statement of a module. It reports a direct `main()` or `run()` call, including a
call passed into another call, such as `sys.exit(run())` or `raise SystemExit(main())`. Calls inside an
`if __name__ == "__main__":` block are allowed.

### Exceptions

- **`__main__.py`**: Python's
  [package entrypoint guidance](https://docs.python.org/3/library/__main__.html#main-py-in-python-packages) recommends a
  bare call in this file, including for zip application entrypoints.

## Fix

The unsafe fix wraps a single-line statement in a main guard. It preserves an inline comment and the file's line
endings. Moving the statement changes import-time behavior, so the fix requires `--fix --unsafe-fixes`. Multiline
statements and statements sharing a line with other code remain reports without a fix.
