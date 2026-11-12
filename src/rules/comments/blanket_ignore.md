Flags suppression comments that name no error code, except the forms Ruff's PGH003, PGH004 and RUF103 report, such as a
bare `# noqa`, `# type: ignore` or `# ruff: ignore`. It covers the rest, such as `# ty: ignore`. A blanket suppression
hides every error on its line or in its file, including errors that appear there later.

## Why is this bad?

A blanket suppression hides every error its checker reports on the line or in the file, including a real bug that lands
there later, and the comment does not say which error it was meant for. Naming the code keeps the other errors visible.
mypy and ty give the same reason for
[codes in `type: ignore`](https://mypy.readthedocs.io/en/stable/error_codes.html#silencing-errors-based-on-error-codes)
and [rule names in `ty: ignore`](https://docs.astral.sh/ty/suppression/#ty-suppression-comments). Ruff's
[blanket-type-ignore](https://docs.astral.sh/ruff/rules/blanket-type-ignore/) also finds such a comment harder to read
and maintain.

## Example

```python report
VALUE = compute()  # ty: ignore
```

Use instead:

```python ok
VALUE = compute()  # ty: ignore[invalid-assignment]
```

## What it does

No fix is offered: the comment does not say which codes it should name.

### Active on

- **`noqa` after an earlier one**: Ruff reads only the first `noqa` of a comment, and a `noqa` it rejects still counts
  as that first one. A later bare `noqa` is therefore not reported by PGH004, and it is reported here.

  ```python report
  import os  # noqa: F401 # noqa
  ```

- **File-level `noqa` after an earlier one**: the same holds for `ruff: noqa` and `flake8: noqa` on a line of its own,
  where Ruff reads only the first one of a comment.

  ```python report
  # flake8: noqa # ruff: noqa
  import os
  ```

- **`simplint: ignore` without codes**: each directive whose brackets are missing or empty.

  ```python report
  OTHER = 2  # simplint: ignore
  ```

  ```python report
  VALUE = 1  # simplint: ignore[]
  ```

- **`ty: ignore`, `pyrefly: ignore` without codes**: each directive whose brackets are missing or empty, with or without
  a reason after it, since the reason does not limit what it hides.

  ```python report
  VALUE = 1  # ty: ignore
  THIRD = 3  # pyrefly: ignore
  FIFTH = 5  # ty: ignore because the stub is wrong
  ```

- **`pyright: ignore` and `type: ignore` with a reason or empty brackets**: PGH003 reports these only when nothing
  follows `ignore` but whitespace or another comment. A reason after them, or empty brackets, leaves them to this rule.

  ```python report
  OTHER = 2  # pyright: ignore because the stub is wrong
  SIXTH = 6  # type: ignore because the stub is wrong
  FOURTH = 4  # type: ignore[]
  ```

- **Each directive of a comment**: a comment can hold several directives, and each one without codes is reported.

  ```python report
  TENTH = 10  # ty: ignore[unresolved-import] # pyrefly: ignore
  ```

### Exceptions

- **Directives that name codes**: `ty: ignore[code]` and `pyright: ignore[code]` limit the suppression to those errors,
  and `noqa: CODE` does the same for Ruff.

  ```python ok
  import os  # noqa: F401

  SEVENTH = 7  # ty:ignore[unresolved-import]
  EIGHTH = 8  # pyright: ignore[reportAttributeAccessIssue]
  ```

- **Forms Ruff reports**: Ruff's PGH004 reports a bare `noqa`, a file-level `ruff: noqa` or `flake8: noqa`, and PGH003
  reports a bare `type: ignore` or `pyright: ignore`. This rule leaves them to Ruff whether or not the project's Ruff
  configuration selects that rule.

  ```python ok
  import os  # noqa

  # flake8: noqa
  VALUE = 1  # type: ignore
  ```

- **`ruff: ignore` and `ruff: file-ignore` without codes**: Ruff's
  [invalid-suppression-comment](https://docs.astral.sh/ruff/rules/invalid-suppression-comment/) (RUF103) reports them,
  and this rule leaves them to Ruff whether or not the project's Ruff configuration selects it.

  ```python ok
  VALUE = 1  # ruff: ignore
  # ruff: file-ignore
  import os
  ```

- **`ruff: ignore` with a space before its codes**: Ruff reads the codes of `ruff: ignore [E501]` and
  `ruff: file-ignore [E501]`, so the directive is not a blanket one.

  ```python ok
  VALUE = 1  # ruff: ignore [E501]
  ```

- **`noqa` comments Ruff rejects**: Ruff warns about a `noqa` whose colon names no code, or whose code runs into other
  text, and applies none of it, so the comment hides nothing.

  ```python ok
  import sys  # noqa: because plugins import it
  ```

- **File-level directives out of place**: Ruff applies `# ruff: noqa` only on a line of its own and
  `# ruff: file-ignore` only at module scope; anywhere else it warns and applies none of it.

  ```python ok
  import os  # ruff: noqa
  ```

- **Directives that take no codes**: coverage.py's pragmas name no error, so they are not blanket suppressions.

  ```python ok
  if DEBUG:  # pragma: no cover
      VALUE = 1
  ```

- **Words that only start with a directive**: a directive must be a whole word, so `ignored` after `ty: ignore` is
  ordinary text.

  ```python ok
  ELEVENTH = 11  # ty: ignored later
  ```

- **Text inside strings**: only comments count, so a directive written in a string literal is not checked.

  ```python ok
  THIRTEENTH = "# ty: ignore"
  ```

## Ruff overlap

Ruff's [blanket-noqa](https://docs.astral.sh/ruff/rules/blanket-noqa/) (PGH004) and
[blanket-type-ignore](https://docs.astral.sh/ruff/rules/blanket-type-ignore/) (PGH003) own the forms that name no code
and that Ruff reads as blanket: the first `noqa` of a comment and the first file-level `ruff: noqa` or `flake8: noqa`,
and a `type: ignore` or `pyright: ignore` with nothing after `ignore`. Ruff's
[invalid-suppression-comment](https://docs.astral.sh/ruff/rules/invalid-suppression-comment/) (RUF103) owns a
`ruff: ignore` or `ruff: file-ignore` whose brackets name no code. This rule leaves those forms to Ruff whether or not
the project's Ruff configuration selects the rule that reports them. It keeps the rest, including the `noqa` that
follows an earlier one in the same comment, and the `ty`, `pyrefly` and `simplint` forms.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rules
  [PGH003](https://docs.astral.sh/ruff/rules/blanket-type-ignore/),
  [PGH004](https://docs.astral.sh/ruff/rules/blanket-noqa/) and
  [RUF103](https://docs.astral.sh/ruff/rules/invalid-suppression-comment/), which report a bare `type: ignore`, the
  first `noqa` of a comment, and `ruff: ignore` without codes; this rule leaves those to them and covers the `ty`,
  `pyright`, `pyrefly` and `simplint` directives and the rest.
