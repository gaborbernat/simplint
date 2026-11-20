Flags a constructor parameter documented by a `:param name:` field in both the class docstring and the `__init__`
docstring.

## Why is this bad?

[Sphinx autodoc](https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html#confval-autoclass_content) shows the
class docstring by default. If the constructor repeats a parameter description, an edit to one copy can leave the other
out of date. When autodoc shows both docstrings, readers see the parameter twice.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
class Reader:
    """Read files.

    :param source: Where to read from.
    """

    def __init__(self, source: str) -> None:
        """Set up.

        :param source: The path to read.
        """
```

Use instead:

```python ok
class Reader:
    """Read files.

    :param source: Where to read from.
    """

    def __init__(self, source: str) -> None:
        """Set up."""
```

## Configuration

- [`docstring-style`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#docstring-markup) in
  `[tool.simplint]`: `"rst"` turns the rule on

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Reader:
      """Read files.

      :param source: Where to read from.
      """

      def __init__(self, source: str) -> None:
          """Set up.

          :param source: The path to read.
          """
  ```

## What it does

Checks classes, when `docstring-style = "rst"`, for constructor parameters documented twice.

### Active on

- **Matching parameter fields.** The rule compares the names in the class and `__init__` docstrings. By default it
  reports the duplicate in `__init__`.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  class Reader:
      """Read files.

      :param source: Where to read from.
      """

      def __init__(self, source: str) -> None:
          """Set up.

          :param source: The path to read.
          """
  ```

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Reader:
      """Read files.

      :param source: Where to read from.
      """

      def __init__(self, source: str, mode: str) -> None:
          """Set up.

          :param mode: How to open it.
          """
  ```

Which copy counts as the repeat depends on the docstring Sphinx renders for the class. The rule resolves it for each
project in this order:

1. a `:class-doc-from:` option on the class's
   [`autoclass`](https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html#directive-autoclass) or
   [`autoexception`](https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html#directive-autoexception)
   directive, in one of the project's pages;
1. `class-doc-from` in `autodoc_default_options` of the project's Sphinx configuration, for a class that such a
   directive names without that option;
1. [`autoclass_content`](https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html#confval-autoclass_content)
   of that configuration;
1. the class docstring, which Sphinx shows when none of these sets another.

A directive names a class by its dotted name, which Sphinx imports as written, or by its simple name, which Sphinx
resolves against the page's `module` or `currentmodule`. A simple name on a page with no module matches a class only
when no other class of the run has that name, and otherwise the rule reports nothing for it. A dotted name that shares
only its tail with the class, such as `a.Reader` for `pkg/a.py` when the docs put `pkg` on
[`sys.path`](https://docs.python.org/3/library/sys.html#sys.path), leaves the class unreported too.

simplint recognizes a `conf.py` beside an index page (`index.rst`, `index.md` or `index.txt`), or one that binds
`extensions`, `master_doc` or `root_doc` by name. A star import or a call to `exec` or
[`globals()`](https://docs.python.org/3/builtins/functions.html#globals) alone does not identify a Sphinx configuration.
In directories it visits, simplint reads `conf.py` even when Ruff excludes that file. A symlink must point inside the
project.

The scan skips hidden directories, excluded directories, vendored configurations (`_vendor`, `vendor`, `third_party`,
`subprojects`) and configurations in nested projects. It respects ignore files when Ruff's `respect-gitignore` is
enabled. If Sphinx configurations disagree on a default, simplint skips classes that depend on that default. An explicit
directive can still determine which copy to keep.

The rule reads plain, annotated and tuple assignments at module level. The three accepted literal values are `class`,
`init` and `both`. For `autodoc_default_options`, it reads the `class-doc-from` key from a dictionary literal.

A computed value, conditional binding, chained assignment, import or dynamic change through `exec` or `globals()` can
leave the setting unknown. The rule also treats settings as unknown if it cannot read or parse `conf.py`, or if the file
exceeds 8 MiB.

- **`class`, or no option**: Sphinx shows the class docstring alone, so the `__init__` copy is the repeat, as above.

- **`both`**: Sphinx shows both docstrings. The rule reports the duplicate in `__init__` and preserves the class
  description.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```rst context file=docs/index.rst
  Index
  =====
  ```

  ```python context file=docs/conf.py
  autoclass_content = "both"
  ```

  ```python report file=model.py
  class Reader:
      """Read files.

      :param source: Where to read from.
      """

      def __init__(self, source: str) -> None:
          """Set up.

          :param source: The path to read.
          """
  ```

- **`init`**: Sphinx shows the `__init__` docstring alone, so the class copy is the repeat. The rule reports it and
  removes it from the class docstring, and the `__init__` copy stays.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```rst context file=docs/index.rst
  Index
  =====
  ```

  ```python context file=docs/conf.py
  autoclass_content = "init"
  ```

  ```python report file=model.py
  class Reader:
      """Read files.

      :param source: Where to read from.
      """

      def __init__(self, source: str) -> None:
          """Set up.

          :param source: The path to read.
          """
  ```

  ```python fixed file=model.py
  class Reader:
      """Read files."""

      def __init__(self, source: str) -> None:
          """Set up.

          :param source: The path to read.
          """
  ```

- **`class-doc-from` on a directive**: an `autoclass` or `autoexception` directive names the class and sets the option,
  which decides the copy of that class ahead of the configuration. Here the configuration sets nothing, and the
  directive makes the class docstring the repeat.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```rst context file=docs/index.rst
  Index
  =====
  ```

  ```rst context file=docs/api.rst
  .. autoclass:: Reader
      :class-doc-from: init
  ```

  ```python report file=model.py
  class Reader:
      """Read files.

      :param source: Where to read from.
      """

      def __init__(self, source: str) -> None:
          """Set up.

          :param source: The path to read.
          """
  ```

- **`autodoc_default_options`**: the key sets the option for each class an `autoclass` or `autoexception` directive
  names without its own option, ahead of `autoclass_content`. A class that an `automodule` documents with `:members:`
  takes `autoclass_content`, since Sphinx gives the key to those directives alone.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```rst context file=docs/index.rst
  Index
  =====
  ```

  ```rst context file=docs/api.rst
  .. autoclass:: Reader
  ```

  ```python context file=docs/conf.py
  autoclass_content = "both"
  autodoc_default_options = {"class-doc-from": "init"}
  ```

  ```python report file=model.py
  class Reader:
      """Read files.

      :param source: Where to read from.
      """

      def __init__(self, source: str) -> None:
          """Set up.

          :param source: The path to read.
          """
  ```

- **Unknown**: a value the rule cannot read, such as one computed when the configuration runs, leaves the docstring
  unknown, so the rule reports nothing.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```rst context file=docs/index.rst
  Index
  =====
  ```

  ```python context file=docs/conf.py
  import os

  autoclass_content = os.environ.get("SPHINX_AUTOCLASS", "class")
  ```

  ```python ok file=model.py
  class Reader:
      """Read files.

      :param source: Where to read from.
      """

      def __init__(self, source: str) -> None:
          """Set up.

          :param source: The path to read.
          """
  ```

### Fix

The fix deletes the field from the `__init__` docstring, with the lines that continue it. Under `init` it deletes the
field from the class docstring instead.

**Unsafe fix:** code may read the docstring through `__doc__`, and the copy the fix deletes may hold details the other
docstring lacks.

```toml config
[tool.simplint]
docstring-style = "rst"
```

```python report
class Reader:
    """Read files.

    :param source: Where to read from.
    """

    def __init__(self, source: str) -> None:
        """Set up.

        :param source: The path to read.
        """
```

```python fixed
class Reader:
    """Read files.

    :param source: Where to read from.
    """

    def __init__(self, source: str) -> None:
        """Set up."""
```

The field must start below the opening quotes in a single triple-quoted string whose text matches the source. If the
field ends the docstring, a comment after the closing quotes also prevents the fix.

## Other tools

- [Ruff PR #23081](https://github.com/astral-sh/ruff/pull/23081) proposes DOC304 for a parameters section in a Google or
  NumPy class docstring.
- By default [pydoclint DOC301](https://jsh9.github.io/pydoclint/violation_codes.html) reports an `__init__` docstring
  even if the class docstring has no duplicate fields.
