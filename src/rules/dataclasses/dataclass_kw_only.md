## What it does

Checks for [dataclasses](https://docs.python.org/3/library/dataclasses.html#dataclasses.dataclass) declared without
`kw_only=True`. The decorator must be spelled `@dataclass` or `@dataclasses.dataclass`. Any value other than a literal
`True`, such as `kw_only=False`, is reported too.

## Why is this bad?

Positional construction ties every caller to the field order, so reordering or inserting a field silently shifts values
into the wrong fields. Keyword-only fields name each value at the call site. The `kw_only` option needs Python 3.10 or
later, so nothing is reported when the target version is older.

[SMP111](SMP111.html) asks for `frozen=True` and `slots=True` on the same decorator.

## Example

```python
@dataclass(frozen=True, slots=True)
class Release:
    name: str
    version: str


Release("simplint", "1.0.0")
```

Use instead:

```python
@dataclass(frozen=True, slots=True, kw_only=True)
class Release:
    name: str
    version: str


Release(name="simplint", version="1.0.0")
```

## Fix safety

The fix is always unsafe: it breaks every caller that passes fields positionally, in any file.

## Options

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python)
