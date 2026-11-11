Flags an `__all__` written as a tuple, or as a list without a trailing comma after its last name.

## Why is this bad?

A trailing comma after the last name makes Ruff's formatter put each name on its own line, unless
[`skip-magic-trailing-comma`](https://docs.astral.sh/ruff/settings/#format_skip-magic-trailing-comma) is set. Adding or
removing an export then changes one line, so the diff shows exactly which name changed.

Using a list in every module gives `__all__` one shape to read and edit, and it is the shape the
[missing-from-all](SMP401.html) fix writes for a new `__all__`. To also sort the names, see Ruff's
[unsorted-dunder-all](https://docs.astral.sh/ruff/rules/unsorted-dunder-all/).

## Example

```python report
__all__ = ("load", "save")
```

Use instead:

```python ok
__all__ = [
    "load",
    "save",
]
```

## Configuration

- [`format.skip-magic-trailing-comma`](https://docs.astral.sh/ruff/settings/#format_skip-magic-trailing-comma); see
  [Settings shared with Ruff](https://simplint.readthedocs.io/en/latest/configuration/project.html#settings-shared-with-ruff)

  ```toml config
  [tool.ruff.format]
  skip-magic-trailing-comma = true
  ```

  ```python ok
  __all__ = ["load", "save"]
  ```

## What it does

The rule checks module-level `=`, annotated and augmented assignments to
[`__all__`](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package) whose value is a list or tuple
literal.

### Active on

- **Lists without a trailing comma**: a list must have a trailing comma after its last name, so the formatter splits it
  one name per line. With Ruff's
  [`skip-magic-trailing-comma`](https://docs.astral.sh/ruff/settings/#format_skip-magic-trailing-comma) set, the
  formatter keeps the list as written, with or without the comma, so the rule does not report it.

  - a plain assignment:

    ```python report
    __all__ = ["load", "save"]
    ```

  - an annotated assignment:

    ```python report
    __all__: list[str] = ["load", "save"]
    ```

- **Tuples**: always reported, whether parenthesized or bare, so every module uses the same list shape. **Unsafe fix:**
  code that concatenates `__all__` with another tuple stops working once it is a list.

  - parenthesized:

    ```python report
    __all__ = ("load", "save")
    ```

  - bare, without brackets:

    ```python report
    __all__ = "run", "stop"
    ```

- **Augmented assignments**: a `+=` of a list or tuple literal is reported like the others, unless a plain assignment to
  `__all__` keeps it a tuple (see Exceptions).

  ```python report
  __all__ = ("load",)
  __all__ += ("save",)
  ```

### Exceptions

- **An empty list**: it has no name to put a comma after.

  ```python ok
  __all__ = []
  ```

- **Other values**, such as `sorted([...])`: they are not a list or tuple literal, so there are no brackets or comma to
  check.

  ```python ok
  __all__ = sorted(["load", "save"])
  ```

- **An augmented assignment while a plain assignment keeps `__all__` a tuple**: `tuple += list` raises, so the `+=` is
  left alone when a plain assignment is not a list or tuple literal, such as `tuple([...])`. A tuple nested in a block
  or with a `noqa` for this rule stays a tuple too, since the fix does not convert it. Other augmented assignments do
  not change the type of `__all__`, so they are not affected.

  ```python ok
  __all__ = tuple(["load"])
  __all__ += ("save",)
  ```

  ```python ok
  import sys

  if sys.platform == "win32":
      __all__ = ("load",)
  else:
      __all__ = ("load", "save")
  __all__ += ("save",)
  ```

  ```python ok
  __all__ = ("load",)  # noqa: SMP405
  __all__ += ("save",)
  ```

- **A list whose closing bracket follows a line break with no comma before it**, such as a last name on its own line
  above `]` with no comma after it: Ruff's
  [missing-trailing-comma](https://docs.astral.sh/ruff/rules/missing-trailing-comma/) reports it, so this rule leaves it
  to Ruff. A tuple written that way, which COM812 reports too, stays here for the fix that turns it into a list.

  ```python ok
  # fmt: off
  __all__ = [
      "load",
      "save"
  ]
  # fmt: on
  ```

### Fix

| Value                                                                              | Fix                                                   | Safe |
| ---------------------------------------------------------------------------------- | ----------------------------------------------------- | :--: |
| list without the trailing comma, no comment, closing bracket after a name          | rewrites it, one name per line, with a trailing comma |  ✓   |
| list without the trailing comma, with a comment and a closing bracket after a name | adds the comma after the last name                    |  ✓   |
| tuple                                                                              | rewrites it as a list                                 |  ✗   |
| empty tuple                                                                        | becomes `[]`                                          |  ✗   |

```python report
__all__ = ["load", "save"]
```

```python fixed
__all__ = [
    "load",
    "save",
]
```

```python report
__all__ = ()
```

```python fixed
__all__ = []
```

The fix keeps the names in their order. The layout depends on how the literal is written:

- **On one line**: it becomes a list with one name per line and a trailing comma, the layout `ruff format` gives it, so
  `("load", "save")` needs no formatter pass. A list on one line is rewritten the same way.

  ```python report
  __all__ = ("load", "save")
  ```

  ```python fixed
  __all__ = [
      "load",
      "save",
  ]
  ```

- **A list whose closing bracket follows the last name**: it is rewritten one name per line, as a list on one line is.

  ```python report
  # fmt: off
  __all__ = [
      "load",
      "save"]
  # fmt: on
  ```

  ```python fixed
  # fmt: off
  __all__ = [
      "load",
      "save",
  ]
  # fmt: on
  ```

- **A list that holds a comment, with its closing bracket after a name**: the fix adds only the comma after the last
  name, after its closing parenthesis if it has one, so the layout and the comment stay.

  ```python report
  # fmt: off
  __all__ = [
      "load",  # the loader
      "save"]
  # fmt: on
  ```

  ```python fixed
  # fmt: off
  __all__ = [
      "load",  # the loader
      "save",]
  # fmt: on
  ```

- **A tuple across several lines**: it keeps its layout; the fix swaps the brackets and adds the comma.

  ```python report
  __all__ = (
      "load",
      "save",
  )
  ```

  ```python fixed
  __all__ = [
      "load",
      "save",
  ]
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rule
  [COM812](https://docs.astral.sh/ruff/rules/missing-trailing-comma/), which reports a missing trailing comma when the
  closing bracket is on its own line and which this rule leaves to it for a list; this rule covers the rest of the
  layout, and keeps a multi-line tuple COM812 also reports, for the fix that turns it into a list. Open issues
  [#22443](https://github.com/astral-sh/ruff/issues/22443) and [#27983](https://github.com/astral-sh/ruff/issues/27983).
- [flake8-dunder-all](https://github.com/python-formate/flake8-dunder-all) reports
  [DALL002](https://github.com/python-formate/flake8-dunder-all) when `__all__` is not a list or tuple of strings. It
  accepts tuples, which this rule reports, and it does not check the trailing comma.
