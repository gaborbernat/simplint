Flags own-line comments used as dividers or section headers. Such a comment groups code by its position in the file, and
nothing keeps that grouping accurate as code moves.

## Why is this bad?

A section comment groups code by its position in the file, and nothing keeps the grouping accurate as code moves. A
named function, class or module groups the same code under a name that tools can check.

## Example

```python report
# ---------- parsing ----------
def parse_header(line: str) -> str:
    return line.split(":", 1)[0]
```

Use instead:

```python ok
def parse_header(line: str) -> str:
    return line.split(":", 1)[0]
```

## What it does

### Active on

- **Divider lines**: comments made only of divider characters (`-`, `=`, `*`, `#`, `~`, `_`, `+`, `/`), at least four of
  them counting the leading `#`, such as `# ---` or `####`; they carry no text, only a visual break.

  ```python report
  # ----
  VALUE = 1
  ```

- **Decorated titles**: comments whose words are framed by divider characters, so the decoration frames them as a
  heading:

  - Three or more of the same divider character at the start or end, such as `# --- helpers`:

    ```python report
    # --- helpers
    def helper() -> None:
        pass
    ```

  - Two or more at both ends, each set apart from the words by a space, such as `# -- helpers --`:

    ```python report
    # -- helpers --
    def other() -> None:
        pass
    ```

- **Markdown-style headings**: comments that open with two or more `#`, such as `## Setup`, the way a document heading
  does.

  ```python report
  ## Setup
  VALUE = 1
  ```

- **Editor regions**: `# region` and `# endregion` markers, which fold code into named sections.

  ```python report
  # region parsing
  VALUE = 1
  # endregion
  ```

### Exceptions

- **Trailing comments**: a comment after code annotates its own line and cannot head a section.

  ```python ok
  VALUE = 1  # ---------- parsing ----------
  ```

- **Comments starting with `#!`**: this is a shebang line.

  ```python ok
  #!/usr/bin/env python3
  VALUE = 1
  ```

- **[PEP 723](https://peps.python.org/pep-0723/) metadata**: the `# ///` lines open and close inline script metadata,
  which tools need in that exact form.

  ```python ok
  # /// script
  # requires-python = ">=3.12"
  # ///
  VALUE = 1
  ```

- **Prose that uses divider characters**: such as `# C++` or `# >>> run()`, they are words, not dividers.

  A run at one end counts only at three or more characters, since two also end words like `C++`. A pair needs a space
  between it and the words, so `# __init__ runs first, before __call__` passes.

  ```python ok
  # C++ is compiled ahead of time
  # >>> run()
  # __init__ runs first, before __call__
  VALUE = 1
  ```

### Fix

The fix is safe, since comments do not affect behavior. What it does depends on the text left once the decoration is
gone:

- **Decoration only, or a title of up to three words**, as in `# === helpers ===`: the fix deletes the comment.

  ```python report
  # === helpers ===
  VALUE = 1
  ```

  ```python fixed
  VALUE = 1
  ```

- **Longer text**, such as an instruction: the fix strips the decoration and keeps a plain comment, so
  `#  *** remove return to update cron` becomes `# remove return to update cron`.

  ```python report
  #  *** remove return to update cron
  VALUE = 1
  ```

  ```python fixed
  # remove return to update cron
  VALUE = 1
  ```

A divider run joined to a word without a space, such as the `++` of `C++`, stays unless it is three characters or
longer.

## Ruff overlap

Ruff's
[multiple-leading-hashes-for-block-comment](https://docs.astral.sh/ruff/rules/multiple-leading-hashes-for-block-comment/)
also reports the `## Setup` form.
