Flags own-line comments used as dividers or section headers.

## Why is this bad?

A section comment groups code by its position in the file, and nothing keeps the grouping accurate as code moves: a
function added below a banner joins its section whether or not it belongs there. A named function, class or module
groups the same code under a name that tools can check. A function named for what its block does
[states that purpose](https://martinfowler.com/bliki/FunctionLength.html) where a banner only labels it.

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

- **Table rules**: divider runs split by spaces underline the columns of a table.

  ```python ok
  #   Config  Resp.
  #   ------  ------
  #   False   False
  VALUE = 1
  ```

- **`##` comments Ruff reports**: a line of a `##` paragraph, or a URL or code after `##`, is not a heading; see Ruff
  overlap.

  ```python ok
  ## The stricter condition would reject
  ## constructions such as
  ## https://yaml.org/spec/1.2.2/
  VALUE = 1
  ```

### Fix

What the fix does depends on the text left once the decoration is gone:

- **Decoration only, or a title of up to three words**, as in `# === helpers ===`: the fix deletes the comment. It is
  safe for a bare divider. **Unsafe fix:** a title's words may still say something about the code below.

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

## Ruff overlap

[E266](https://docs.astral.sh/ruff/rules/multiple-leading-hashes-for-block-comment/) and
[E265](https://docs.astral.sh/ruff/rules/no-space-after-block-comment/), preview rules, report a prose heading such as
`## Remove this return to update the cron job`, whose fix here would be the same, so this rule leaves it to them whether
or not the project enables preview. The same goes for a short line inside a `##` paragraph and for a URL or code after
`##`, which are no headings. Short headings such as `## Setup`, decorated dividers and regions stay with this rule,
since they report a section divider to replace with a function or remove, not the hash count E266 reports.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of preview rules
  [E265](https://docs.astral.sh/ruff/rules/no-space-after-block-comment/) and
  [E266](https://docs.astral.sh/ruff/rules/multiple-leading-hashes-for-block-comment/), which reformat some of the same
  comments, such as `## Setup` or `#---`; this rule leaves the plain prose ones to them (see Ruff overlap).
