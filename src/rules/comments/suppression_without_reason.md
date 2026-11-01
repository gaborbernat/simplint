## What it does

Checks for suppression comments that give no reason:

- Ruff's [`noqa` and `ruff: noqa`](https://docs.astral.sh/ruff/linter/#error-suppression), and its `ruff: ignore[...]`
  and `ruff: file-ignore[...]` comments
- simplint's own `simplint: ignore[...]`
- [`type: ignore`](https://typing.python.org/en/latest/spec/directives.html#type-ignore-comments)
- [`ty: ignore`](https://docs.astral.sh/ty/suppression/)
- [`pyright: ignore`](https://microsoft.github.io/pyright/#/comments)
- [`pyrefly: ignore`](https://pyrefly.org/en/docs/error-suppressions/)
- coverage.py's [`pragma: no cover`](https://coverage.readthedocs.io/en/latest/excluding.html) and
  [`pragma: no branch`](https://coverage.readthedocs.io/en/latest/branch.html)

A reason is any text after the error codes, or another `#` comment on the same line. A `noqa` code looks like `F401`:
uppercase letters followed by digits. The `ignore` comments take their codes in brackets, with or without a space after
the colon. A directive must be a whole word: one followed by a letter, digit, `-` or `_`, such as `# noqa-ish`, is not a
directive.

A `noqa`, `ruff: noqa`, `type: ignore`, `ty: ignore` or `pyright: ignore` that names no error code is reported even when
it has a reason, since it silences every error on the line or file. Examples are a bare `# noqa`, a `# type: ignore`
without brackets, or `# noqa:` followed by prose. Ruff's
[blanket-noqa (PGH004)](https://docs.astral.sh/ruff/rules/blanket-noqa/) and
[blanket-type-ignore (PGH003)](https://docs.astral.sh/ruff/rules/blanket-type-ignore/) report blanket `noqa` and
`type: ignore` comments. A blanket `# noqa` does not suppress this rule.

## Why is this bad?

A suppression outlives the problem it hid. Without a reason, a later reader cannot tell whether the error is a false
positive, a known stub gap or a bug someone meant to fix, so nobody dares remove it.

## Example

```python
import os  # noqa: F401

VALUE = os.sep  # type: ignore[attr-defined]
```

Use instead:

```python
import os  # noqa: F401  # re-exported for plugins

VALUE = os.sep  # type: ignore[attr-defined]  # stub omits sep on this platform
```

## Fix safety

No fix is offered: only the author knows why the error was suppressed.
