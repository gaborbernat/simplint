Flags suppression comments, such as `# noqa: F401` or `# type: ignore[...]`, that give no reason.

## Why is this bad?

A suppression outlives the problem it hid. Without a reason, a later reader cannot tell whether the error is a false
positive, a known stub gap or a bug someone meant to fix, so nobody dares remove it.

## Example

```python report
import os  # noqa: F401

VALUE = os.sep  # type: ignore[attr-defined]
```

Use instead:

```python ok
import os  # noqa: F401  # re-exported for plugins

VALUE = os.sep  # type: ignore[attr-defined]  # stub omits sep on this platform
```

## What it does

No fix is offered: only the author knows why the error was suppressed.

### Active on

Each of these suppression comments is reported when it has no **reason**: any text after the error codes, or another `#`
comment on the same line, or a comment on the line above. A comment with neither codes nor a reason is reported here for
the missing reason, and by [blanket-ignore](SMP004.html) for the missing codes.

- **Ruff**: [error suppression](https://docs.astral.sh/ruff/linter/#error-suppression) comments, in these forms:

  - `noqa`: a trailing comment with error codes. A code looks like
    [unused-import](https://docs.astral.sh/ruff/rules/unused-import/): uppercase letters followed by digits.

    ```python report
    import os  # noqa: F401
    ```

  - `ruff: noqa`: the same suppression with the `ruff:` prefix, on a line of its own.

    ```python report
    # ruff: noqa: F401
    import os
    ```

  - `ruff: ignore[...]`: a trailing comment with codes in brackets.

    ```python report
    import sys  # ruff: ignore[unused-import]
    ```

  - `ruff: file-ignore[...]`: a comment on a line of its own with codes in brackets.

    ```python report
    # ruff: file-ignore[unused-import]
    import sys
    ```

- **simplint**: its own `simplint: ignore[...]`.

  ```python report
  VALUE = 1  # simplint: ignore[SMP101]
  ```

- **Type checkers**: [`type: ignore`](https://typing.python.org/en/latest/spec/directives.html#type-ignore-comments),
  [`ty: ignore`](https://docs.astral.sh/ty/suppression/),
  [`pyright: ignore`](https://microsoft.github.io/pyright/#/comments), and
  [`pyrefly: ignore`](https://pyrefly.org/en/docs/error-suppressions/). The `ignore` comments take their codes in
  brackets, with or without a space after the colon.

  ```python report
  VALUE = os.sep  # type: ignore[attr-defined]
  OTHER = 1  # ty: ignore[unresolved-import]
  THIRD = 2  # pyright: ignore[reportGeneralTypeIssues]
  FOURTH = 3  # pyrefly: ignore[bad-assignment]
  ```

- **coverage.py**: [`pragma: no cover`](https://coverage.readthedocs.io/en/latest/excluding.html) and
  [`pragma: no branch`](https://coverage.readthedocs.io/en/latest/branch.html).

  ```python report
  if DEBUG:  # pragma: no cover
      VALUE = 1
  ```

### Exceptions

- **Suppressions with a reason**: any text after the codes, or another comment on the same line, counts as the reason.

  ```python ok
  import os  # noqa: F401  # re-exported for plugins

  VALUE = os.sep  # type: ignore[attr-defined]  # stub omits sep on this platform
  ```

- **A reason on the line above**: a comment on the line directly above a trailing suppression counts, as do the comment
  lines in a run that ends there. The reason may take several lines, but a blank line or a line of code ends the run,
  and a suppression on a line of its own takes no reason from above.

  ```python ok
  # the plugin under test expects a str, so the wrong type is deliberate
  value = 123  # ty: ignore[invalid-argument-type]
  ```

- **Words that only start with a directive**: a directive must be a whole word, so one followed by a letter, digit, `-`
  or `_`, such as `# noqa-ish`, is ordinary text, not a suppression.

  ```python ok
  NOTE = 9  # noqa-ish wording is prose
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet. Closed issue
  [#10070](https://github.com/astral-sh/ruff/issues/10070) (fixed by pull request
  [#21294](https://github.com/astral-sh/ruff/pull/21294)) only lets `--add-noqa` write a reason; no Ruff rule reports a
  suppression without one.
