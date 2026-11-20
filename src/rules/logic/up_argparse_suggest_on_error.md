Flags the explicit Python 3.15 default `suggest_on_error=True` in
[`argparse.ArgumentParser()`](https://docs.python.org/3/library/argparse.html#argparse.ArgumentParser) calls.

## Why is this bad?

Python 3.15 enables [suggestions](https://docs.python.org/3/library/argparse.html#suggest-on-error) for a mistyped
string choice or subparser name by default. Pass `suggest_on_error=False` to turn them off.

## Example

```toml config
[project]
requires-python = ">=3.15"
```

```python report
import argparse

parser = argparse.ArgumentParser(prog="tool", suggest_on_error=True)
```

Use instead:

```python ok
import argparse

parser = argparse.ArgumentParser(prog="tool")
```

## Configuration

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python);
  see [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

  ```toml config
  [tool.ruff]
  target-version = "py315"
  ```

  ```python report
  import argparse

  parser = argparse.ArgumentParser(prog="tool", suggest_on_error=True)
  ```

## What it does

### Active on

- **`suggest_on_error=True` in calls of `argparse.ArgumentParser`**, by its qualified name or by a name imported from
  `argparse`. **Safe fix:** the keyword is removed with the comma that separates it from its neighbor.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python report
  import argparse

  parser = argparse.ArgumentParser(prog="tool", suggest_on_error=True)
  ```

  ```python fixed
  import argparse

  parser = argparse.ArgumentParser(prog="tool")
  ```

### Exceptions

- **`suggest_on_error=False`**: it turns the suggestions off, which the default no longer does.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python ok
  import argparse

  parser = argparse.ArgumentParser(prog="tool", suggest_on_error=False)
  ```

- **A value that is not the literal `True`**, such as `suggest_on_error=verbose`: its value is not known.

  ```toml config
  [project]
  requires-python = ">=3.15"
  ```

  ```python ok
  import argparse

  parser = argparse.ArgumentParser(prog="tool", suggest_on_error=verbose)
  ```

### Fix

simplint reports the keyword without a fix if the call contains a comment.

```toml config
[project]
requires-python = ">=3.15"
```

```python report
import argparse

parser = argparse.ArgumentParser(
    prog="tool",  # the CLI name
    suggest_on_error=True,
)
```

A parenthesized positional argument stays a valid call when the keyword after it is removed.

```toml config
[project]
requires-python = ">=3.15"
```

```python report
import argparse

parser = argparse.ArgumentParser(("prog"), suggest_on_error=True)
```

```python fixed
import argparse

parser = argparse.ArgumentParser("prog")
```

## Other tools

Refurb's [FURB120](https://github.com/dosisod/refurb/blob/master/docs/checks.md#furb120-use-implicit-default) also
checks for arguments that repeat a default.
