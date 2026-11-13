Flags a constructor parameter documented by a `:param name:` field in both the class docstring and the `__init__`
docstring.

## Why is this bad?

[autodoc](https://www.sphinx-doc.org/en/master/usage/extensions/autodoc.html#confval-autoclass_content) shows the class
docstring by default, so the copy in `__init__` is read only by those who open the source, and the two copies drift
apart as each is edited on its own.

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

- **`:param name:` fields in the `__init__` docstring**: reported when the class docstring also has a `:param name:`
  field for the same constructor parameter.

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

A Sphinx configuration is a `conf.py` beside an index page (`index.rst`, `index.md` or `index.txt`), or one that assigns
`extensions`, `master_doc` or `root_doc`. A wildcard binding, such as a star import, `exec` or `globals()`, does not
count as an assignment. The rule reads it whether or not Ruff excludes it, and follows a symlink whose target lies in
the project. It skips vendored directories (`_vendor`, `vendor`, `third_party`, `subprojects`), other projects below the
root, and gitignored files. The Sphinx configurations of one project must agree on the option; when they do not, the
rule reports nothing for that project.

The rule reads each option from a top-level assignment, plain, annotated, or as one target of a tuple assignment, and a
`class-doc-from` key from a dict literal. The docstring is unknown, and the rule reports nothing for the class, when the
option holds a value other than `class`, `init` or `both`, or a value that is not a literal, when the name is bound
another way (in a block, a chained assignment, an import, a star import, `exec`, or through
[`globals()`](https://docs.python.org/3/builtins/functions.html#globals)), and when the `conf.py` cannot be read, does
not parse, or is over 8 MiB.

The examples below show each case:

- **`class`, or no option**: Sphinx shows the class docstring alone, so the `__init__` copy is the repeat, as above.

- **`both`**: Sphinx shows both docstrings, so the parameter appears twice whichever copy goes. The rule still reports
  the `__init__` copy and removes it, and the class copy stays.

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

No fix is offered when escapes in the docstring make its text differ from the source.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet. Open pull request
  [#23081](https://github.com/astral-sh/ruff/pull/23081) (DOC304) reports any parameters section in a Google or NumPy
  class docstring; this rule reports reST `:param:` fields documented in both the class and `__init__`.
- [pydoclint](https://github.com/jsh9/pydoclint) reports any `__init__` docstring as
  [DOC301](https://jsh9.github.io/pydoclint/violation_codes.html), not only the parameters documented in both places.
