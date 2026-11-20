Flags missing, extraneous and misused `:returns:` and `:yields:` fields in reStructuredText docstrings.

## Why is this bad?

A return annotation such as `int` cannot tell a caller whether the value is a byte count or an exit status. For a
generator, describe what each iteration yields. A generator can also return a final value through
[`StopIteration.value`](https://docs.python.org/3/builtins/exceptions.html#StopIteration.value), which is distinct from
the items it yields.
[Python's return statement](https://docs.python.org/3/reference/simple_stmts.html#the-return-statement) and
[numpydoc's generator convention](https://numpydoc.readthedocs.io/en/latest/format.html#yields) explain the distinction.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
project-kind = "library"
```

```python report
def total(values: list[int]) -> int:
    """Add the values.

    :param values: The values.
    """
    return sum(values)
```

Use instead:

```python ok
def total(values: list[int]) -> int:
    """Add the values.

    :param values: The values.
    :returns: Their sum.
    """
    return sum(values)
```

## Conflicts with Ruff

Ruff's [docstring-missing-returns](https://docs.astral.sh/ruff/rules/docstring-missing-returns/) and
[docstring-missing-yields](https://docs.astral.sh/ruff/rules/docstring-missing-yields/) read Google and numpydoc
sections only, so they report the `:returns:` and `:yields:` fields this rule asks for as missing sections. simplint
stops with an error when either applies to the same file, so turn the Ruff rules off, or this one.

## Configuration

- [`docstring-style`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#docstring-markup) in
  `[tool.simplint]`: `"rst"` turns the rule on

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def total(values: list[int]) -> int:
      """Add the values.

      :param values: The values.
      :yields: Each value.
      """
      return sum(values)
  ```

## What it does

Checks the `:returns:` and `:yields:` fields of reStructuredText docstrings, when `docstring-style = "rst"`. It checks
for missing fields on the [public API](SMP903.html), including definitions named by autodoc. It also checks for
misplaced fields on internal functions and classes.

### Active on

- **Public functions that return a value without a `:returns:` field**: the return annotation is not `None`, `NoReturn`
  or `Never`, or, without one, the function returns something other than `None`. No fix is offered: the description is
  for the author to write, and [rst-empty-field](SMP914.html) reports an empty `:returns:` until they do.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def total(values: list[int]) -> int:
      """Add the values.

      :param values: The values.
      """
      return sum(values)
  ```

- **Public generators without a `:yields:` field**: no fix is offered, for the same reason.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def numbers(count: int):
      """Count up.

      :param count: How far.
      """
      yield from range(count)
  ```

- **`:returns:` on a function that returns nothing**: its annotation says so, or, without one, it has no `return` with a
  value and its body is more than a placeholder such as `...`, as in an abstract method. The fix deletes the `:returns:`
  and `:rtype:` fields with the lines that continue them.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def log(message: str) -> None:
      """Write the message.

      :param message: The text.
      :returns: Nothing.
      """
      print(message)
  ```

  ```python fixed
  def log(message: str) -> None:
      """Write the message.

      :param message: The text.
      """
      print(message)
  ```

  A quoted annotation, such as `-> "None"`, counts the same as its unquoted form:

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def log(message: str) -> "None":
      """Write the message.

      :param message: The text.
      :returns: Nothing.
      """
      print(message)
  ```

- **`:returns:` and no `:yields:` on a generator**: the fix renames the first return field to `:yields:`, keeping its
  description. A generator with both fields keeps them.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def numbers(count: int):
      """Count up.

      :param count: How far.
      :returns: Each number.
      """
      yield from range(count)
  ```

  ```python fixed
  def numbers(count: int):
      """Count up.

      :param count: How far.
      :yields: Each number.
      """
      yield from range(count)
  ```

- **`:yields:` on a function that is not a generator**: the fix deletes the `:yields:` and `:ytype:` fields.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def total(values: list[int]) -> int:
      """Add the values.

      :param values: The values.
      :returns: The sum.
      :yields: Each value.
      """
      return sum(values)
  ```

  ```python fixed
  def total(values: list[int]) -> int:
      """Add the values.

      :param values: The values.
      :returns: The sum.
      """
      return sum(values)
  ```

- **A `:returns:`, `:rtype:`, `:yields:` or `:ytype:` field in the docstring of a class or a property**: no fix is
  offered, since that text belongs in the description, and moving it there is for the author.

  - The docstring of a class, which describes the class

    ```toml config
    [project]
    name = "demo"

    [build-system]
    requires = []

    [tool.simplint]
    docstring-style = "rst"
    ```

    ```python report
    class Engine:
        """Run jobs.

        :returns: The engine.
        """
    ```

  - The docstring of a property, whose text describes the value it holds

    ```toml config
    [project]
    name = "demo"

    [build-system]
    requires = []

    [tool.simplint]
    docstring-style = "rst"
    ```

    ```python report
    class Engine:
        """Run jobs."""

        @property
        def speed(self) -> int:
            """The speed.

            :returns: Units per second.
            """
            return 1
    ```

### Exceptions

- **`__init__`**: a constructor returns nothing to document.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  class Engine:
      """Run jobs."""

      def __init__(self, speed: int) -> None:
          """Set up.

          :param speed: How fast it runs.
          """
          self.speed = speed
  ```

- **A `yield` inside a lambda**: the lambda is a generator of its own, so the function around it is not one.

  ```toml config
  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def count(values: list[int]) -> int:
      """Count the values.

      :param values: The inputs.
      :returns: How many there are.
      """
      probe = lambda: (yield)  # noqa: E731
      return len(values) + bool(probe)
  ```

- **A body that is only a placeholder**: a body such as `...`, `pass` or `raise NotImplementedError`, as in an abstract
  method, says nothing of the return or the yields, so neither a missing `:returns:` nor a `:yields:` is reported for
  it.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok file=total.py
  def total(values: list[int]):
      """Add the values.

      :param values: The values.
      """
      ...
  ```

  ```python ok file=rows.py
  from collections.abc import Iterator


  def rows() -> Iterator[str]:
      """Read the rows.

      :yields: Each row.
      """
      ...
  ```

### Fix

**Unsafe fix:** each fix changes the docstring, and code may read it through `__doc__`.

The field must start below the opening quotes in a single triple-quoted string whose text matches the source. For a
deletion, if the field ends the docstring, a comment after the closing quotes also prevents the fix.

## Other tools

- Ruff's [DOC201](https://docs.astral.sh/ruff/rules/docstring-missing-returns/) and
  [DOC402](https://docs.astral.sh/ruff/rules/docstring-missing-yields/) check for missing Returns and Yields sections.
  [Ruff PR #13286](https://github.com/astral-sh/ruff/pull/13286) proposes support for Sphinx fields.
- [pydoclint DOC201, DOC202, DOC402 and DOC403](https://jsh9.github.io/pydoclint/violation_codes.html) check missing and
  extra return and yield sections. [darglint DAR201 and DAR202](https://github.com/terrencepreilly/darglint) check
  return sections.
