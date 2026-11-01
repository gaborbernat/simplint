## What it does

Checks for own-line comments used as dividers or section headers. A comment is reported when it:

- is made only of divider characters (`-`, `=`, `*`, `#`, `~`, `_`, `+`, `/`), at least four of them counting the
  leading `#`, such as `# ---` or `####`
- starts or ends with three or more of the same divider character, such as `# --- helpers`
- starts and ends with two or more set apart from the words by a space, such as `# -- helpers --`, but not
  `# __init__ runs first, before __call__`
- opens with two or more `#`, such as `## Setup`
- is a `# region` or `# endregion` marker

Trailing comments are skipped. So are comments starting with `#!` and the `# ///` lines that open and close
[PEP 723](https://peps.python.org/pep-0723/) inline script metadata. Comments such as `# C++` or `# >>> run()` are not
dividers. Ruff's
[multiple-leading-hashes-for-block-comment (E266)](https://docs.astral.sh/ruff/rules/multiple-leading-hashes-for-block-comment/)
also reports the `## Setup` form.

## Why is this bad?

A section comment groups code by its position in the file, and nothing keeps the grouping accurate as code moves. A
named function, class or module groups the same code under a name that tools can check.

## Example

```python
# ---------- parsing ----------
def parse_header(line: str) -> str:
    return line.split(":", 1)[0]
```

Use instead:

```python
def parse_header(line: str) -> str:
    return line.split(":", 1)[0]
```

## Fix safety

The fix deletes a comment that is only decoration, or decoration around a title of up to three words, as in
`# === helpers ===`. A longer text, such as an instruction, is kept: the fix strips the decoration and leaves a plain
comment, so `#  *** remove return to update cron` becomes `# remove return to update cron`. A divider run joined to a
word without a space, such as the `++` of `C++`, stays unless it is three characters or longer. The fix is always safe,
since comments do not affect behavior.
