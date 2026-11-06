Flags `suggest_on_error=True` in `argparse.ArgumentParser()` calls, which Python 3.15 makes the default.

## Why is this bad?

From Python 3.15 `argparse` suggests the closest valid choice on a mistyped argument without asking, so
`suggest_on_error=True` repeats the default. Before Python 3.14 the keyword does not exist at all, so the call fails
there with `TypeError`.

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

No fix is offered when the call has a comment, which the removal would drop. The keyword is still reported.

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

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
