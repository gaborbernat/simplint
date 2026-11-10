Flags a trailing `main()` call that runs when its module is imported.

## Why is this bad?

Importing a module executes its top-level statements. A command that calls `main()` during import can run unexpectedly
when another module imports its functions or tests inspect it.
[Python's `__main__` documentation](https://docs.python.org/3/library/__main__.html#idiomatic-usage) recommends a guard
for ordinary modules that can run as scripts.

## Example

```python report file=command.py
def main() -> None:
    print("ready")


main()
```

Use instead:

```python ok file=command.py
def main() -> None:
    print("ready")


if __name__ == "__main__":
    main()
```

## What it does

The rule checks the final top-level statement of a module. It reports a direct `main()` call and a call passed into an
exit wrapper, such as `sys.exit(main())` or `raise SystemExit(main())`. Calls inside an `if __name__ == "__main__":`
block are allowed. It offers no fix because moving a call into a guard can change import-time behavior.

### Exceptions

- **`__main__.py`**: Python's
  [package entrypoint guidance](https://docs.python.org/3/library/__main__.html#main-py-in-python-packages) recommends a
  bare call in this file, including for zip application entrypoints.
