## What it does

Checks for `+` chains that join a string literal or
[f-string](https://docs.python.org/3/reference/lexical_analysis.html#f-strings) with other values. Chains that contain a
bytes literal are not reported.

## Why is this bad?

A chain of `+` scatters the text across quotes and operators, and every non-string operand needs an explicit `str()`. An
f-string ([PEP 498](https://peps.python.org/pep-0498/)) shows the final text in one piece.

## Example

```python
message = "Hello, " + name + "!"
```

Use instead:

```python
message = f"Hello, {name}!"
```

## Fix safety

The fix is always unsafe. `+` raises [`TypeError`](https://docs.python.org/3/builtins/exceptions.html#TypeError) for a
non-string operand and honors a custom [`__add__`](https://docs.python.org/3/reference/datamodel.html#object.__add__),
while an f-string formats any value.

The fix copies each non-literal operand into a placeholder as written. A call to the builtin `str`, `repr` or `ascii`
becomes the matching conversion, as Ruff's
[RUF010](https://docs.astral.sh/ruff/rules/explicit-f-string-type-conversion/) asks, so `"n=" + str(n)` becomes
`f"n={n!s}"`. A chain of plain string literals becomes one plain string.

No fix is offered when:

- a literal is implicitly concatenated, triple-quoted, raw or `u`-prefixed;
- the literals use different quote characters;
- a comment sits between the operands, since the f-string has no place for it;
- a non-literal operand spans several lines or contains a backslash, `#`, `:`, `!`, `{` or `}`;
- the target Python version is older than 3.12 and a non-literal operand contains the string's quote character, which
  f-strings allow only from 3.12 on ([PEP 701](https://peps.python.org/pep-0701/#considerations-of-quote-reuse)).

## Options

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python)
