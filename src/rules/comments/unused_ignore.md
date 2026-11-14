Reports selectors in `# simplint: ignore[...]` comments that no longer suppress a report under the active rule settings.
Enable it with `--extend-select SMP009` when checking for obsolete ignores.

## Why is this bad?

An obsolete ignore makes it harder to tell which exceptions still matter. It can also hide a new report if the same rule
starts applying to that line later. Other checkers report obsolete ignores too: Ruff's
[unused-noqa](https://docs.astral.sh/ruff/rules/unused-noqa/), mypy's
[`--warn-unused-ignores`](https://mypy.readthedocs.io/en/stable/command_line.html#cmdoption-mypy-warn-unused-ignores)
and pyright's
[`reportUnnecessaryTypeIgnoreComment`](https://microsoft.github.io/pyright/#/configuration?id=reportUnnecessaryTypeIgnoreComment).

## Example

```python report
value = 1  # simplint: ignore[single-letter-name]
```

Use instead:

```python ok
value = 1
```

## What it does

The rule tests removals one selector at a time, then offers one safe fix for the file.

### Active on

- **Selectors no report needs**: a selector whose removal exposes no report is reported, and the fix removes it.

  ```python report
  total = sum(values)  # simplint: ignore[single-letter-name]
  ```

  ```python fixed
  total = sum(values)
  ```

- **Several selectors**: each selector is tested on its own, and the fix keeps those that still suppress reports.

  ```python report select=SMP101
  x = sum(values)  # simplint: ignore[single-letter-name, meaningless-name]
  ```

  ```python fixed
  x = sum(values)  # simplint: ignore[single-letter-name]
  ```

- **A comment after the directive**: when its last selector is unused, the directive goes with the prose comment that
  gives its reason, while another tool's directive later on the line, such as `# noqa: E501`, stays.

  ```python report
  value = 1  # simplint: ignore[single-letter-name]  # wire order  # noqa: E501
  ```

  ```python fixed
  value = 1  # noqa: E501
  ```

- **Selectors for rules outside a narrow `--select`**: the rule checks the rules currently selected for the file, so
  selectors for rules outside that selection can appear unused. Use the project's normal settings when cleaning up
  ignores.

  ```python report select=SMP009
  for i in range(3):  # simplint: ignore[single-letter-name]
      print(i)
  ```

### Exceptions

- **Selectors whose removal exposes a report**: each proposed removal is re-linted, and the selector stays when removing
  it exposes any report, including one from a different rule.

  ```python ok select=SMP101
  def scale(values: list[float]) -> list[float]:
      x = 2  # simplint: ignore[single-letter-name]
      return [value * x for value in values]
  ```
