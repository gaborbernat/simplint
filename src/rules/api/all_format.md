Flags an `__all__` written as a tuple, or as a list without a trailing comma after its last name.

## Why is this bad?

With Ruff's default formatter settings, a trailing comma puts each export on its own line. Adding or removing a name
then changes one line.
[Black](https://black.readthedocs.io/en/stable/the_black_code_style/current_style.html#how-black-wraps-lines) uses the
same convention, and [PEP 8](https://peps.python.org/pep-0008/#when-to-use-trailing-commas) recommends it for
collections that grow over time.

A list supports `.append`, `.extend` and `.remove`, three ways to update `__all__` that
[type checkers recognize](https://typing.python.org/en/latest/spec/distributing.html#library-interface-public-and-private-symbols).
The [Python tutorial](https://docs.python.org/3/tutorial/modules.html#importing-from-a-package) uses a list, and
[missing-from-all](SMP401.html) creates one. Ruff's
[unsorted-dunder-all](https://docs.astral.sh/ruff/rules/unsorted-dunder-all/) checks the order of the names.

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
  [`skip-magic-trailing-comma`](https://docs.astral.sh/ruff/settings/#format_skip-magic-trailing-comma) set, Ruff may
  put a short list on one line even if it has a trailing comma. simplint skips the trailing-comma check for lists under
  that setting.

  - a plain assignment:

    ```python report
    __all__ = ["load", "save"]
    ```

  - an annotated assignment:

    ```python report
    __all__: list[str] = ["load", "save"]
    ```

- **Tuples**: always reported, whether parenthesized or bare, so every module uses the same list shape. **Unsafe fix:**
  code that concatenates `__all__` with another tuple stops working once it is a list. The fix is left off when another
  module imports `__all__`, or a test reads it by name, as
  [`getattr(module, "__all__")`](https://docs.python.org/3/builtins/functions.html#getattr) does, since either may rely
  on the tuple.

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

- **Other values**, such as [`sorted([...])`](https://docs.python.org/3/builtins/functions.html#sorted): they are not a
  list or tuple literal, so there are no brackets or comma to check.

  ```python ok
  __all__ = sorted(["load", "save"])
  ```

- **An augmented assignment while a plain assignment keeps `__all__` a tuple**: `tuple += list` raises, so the `+=` is
  left alone when a plain assignment is not a list or tuple literal, such as `tuple([...])`. A tuple nested in a block
  or with a `noqa` for this rule stays a tuple too, since the fix does not convert it.

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
  to Ruff while the project's Ruff configuration selects COM812. Without it, this rule reports the list: `ruff format`
  joins a list that fits on one line, where COM812 no longer reports it, and the next run would bring it back here. A
  tuple written that way, which COM812 reports too, stays here for the fix that turns it into a list.

  ```toml config
  [tool.ruff.lint]
  select = ["COM812"]
  ```

  ```python ok
  # fmt: off
  __all__ = [
      "load",
      "save"
  ]
  # fmt: on
  ```

### Fix

| Value                                                                                | Fix                                                   | Safe |
| ------------------------------------------------------------------------------------ | ----------------------------------------------------- | :--: |
| list without the trailing comma, no comment, closing bracket after a name            | rewrites it, one name per line, with a trailing comma |  ✓   |
| list without the trailing comma, with a comment or a closing bracket on its own line | adds the comma after the last name                    |  ✓   |
| tuple                                                                                | rewrites it as a list                                 |  ✗   |
| empty tuple                                                                          | becomes `[]`                                          |  ✗   |

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

The fix preserves the order of the names. The examples below use the default formatter settings. With
`skip-magic-trailing-comma` set, a one-line tuple becomes a one-line list.

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

- **A parenthesized tuple across several lines** keeps its layout. The fix changes the parentheses to brackets and adds
  a missing trailing comma unless `skip-magic-trailing-comma` is set.

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

- Ruff's [COM812](https://docs.astral.sh/ruff/rules/missing-trailing-comma/) checks for a missing comma before a closing
  bracket on its own line. When COM812 is enabled, simplint leaves those lists to Ruff. It still reports tuples so it
  can change them to lists. Ruff issues [#22443](https://github.com/astral-sh/ruff/issues/22443) and
  [#27983](https://github.com/astral-sh/ruff/issues/27983) request multiline export lists and a warning for tuples,
  respectively.
- [flake8-dunder-all](https://github.com/python-formate/flake8-dunder-all) reports
  [DALL002](https://github.com/python-formate/flake8-dunder-all) when `__all__` is not a list or tuple of strings. It
  accepts tuples, which this rule reports, and it does not check the trailing comma.
