Flags suppression comments that name no error code: a bare `# noqa`, a `# ruff: noqa` or `# type: ignore`, and an
`ignore` comment without brackets. A blanket suppression hides every error on its line or in its file, including errors
that appear there later.

## Why is this bad?

A blanket suppression hides every error on its line, or in its file, including errors that appear there later. A real
bug that lands on the same line passes unnoticed, and the comment never says which error it was meant for.

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

- **`noqa` without codes**: a bare `# noqa`, with or without text after it:

  ```python report
  import os  # noqa
  import sys  # noqa because plugins import it
  ```

- **File-level `noqa` without codes**: each of these forms hides every error in the file:

  - `# ruff: noqa` on a line of its own:

    ```python report
    # ruff: noqa
    import os
    ```

  - `# flake8: noqa` on a line of its own:

    ```python report
    # flake8: noqa
    import os
    ```

  - `# ruff: file-ignore` without brackets:

    ```python report
    # ruff: file-ignore
    import os
    ```

- **`ruff: ignore` and `simplint: ignore` without codes**: each directive whose brackets are missing or empty.

  ```python report
  VALUE = 1  # ruff: ignore
  OTHER = 2  # simplint: ignore
  ```

- **`ty: ignore`, `pyright: ignore`, `pyrefly: ignore` and `type: ignore` without codes**: each directive whose brackets
  are missing or empty, with or without a reason after it, since the reason does not limit what it hides.

  ```python report
  VALUE = 1  # ty: ignore
  OTHER = 2  # pyright: ignore
  THIRD = 3  # pyrefly: ignore
  FIFTH = 5  # ty: ignore because the stub is wrong
  SIXTH = 6  # type: ignore
  ```

- **Each directive of a comment**: a comment can hold several directives, and each one without codes is reported.

  ```python report
  TENTH = 10  # ty: ignore[unresolved-import] # pyright: ignore
  ```

### Exceptions

- **Directives that name codes**: `ty: ignore[code]` and `pyright: ignore[code]` limit the suppression to those errors,
  and `noqa: CODE` does the same for Ruff.

  ```python ok
  import os  # noqa: F401

  SEVENTH = 7  # ty:ignore[unresolved-import]
  EIGHTH = 8  # pyright: ignore[reportAttributeAccessIssue]
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

Ruff's [blanket-type-ignore](https://docs.astral.sh/ruff/rules/blanket-type-ignore/) and
[blanket-noqa](https://docs.astral.sh/ruff/rules/blanket-noqa/) report the `type: ignore` and `noqa` forms of this rule,
so a project running both sees two reports for the same comment.
