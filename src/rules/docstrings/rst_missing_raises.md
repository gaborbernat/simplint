Flags exceptions a public API function raises itself that its reStructuredText docstring leaves without a
`:raises Error:` field.

## Why is this bad?

A caller may need to handle a missing file differently from an invalid file. Documenting the exception and its cause
tells the caller which handler to write. [PEP 257](https://peps.python.org/pep-0257/#multi-line-docstrings) recommends
documenting applicable exceptions.
[Google's style guide](https://google.github.io/styleguide/pyguide.html#383-functions-and-methods) limits this to
exceptions relevant to the interface.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
project-kind = "library"
```

```python report
def parse(text: str) -> int:
    """Parse a number.

    :param text: The text.
    :returns: The number.
    """
    if not text:
        raise ValueError("empty")
    return int(text)
```

Use instead:

```python ok
def parse(text: str) -> int:
    """Parse a number.

    :param text: The text.
    :returns: The number.
    :raises ValueError: When the text is empty.
    """
    if not text:
        raise ValueError("empty")
    return int(text)
```

## Conflicts with Ruff

Ruff's [docstring-missing-exception](https://docs.astral.sh/ruff/rules/docstring-missing-exception/) reads Google and
numpydoc sections only, so it reports the `:raises:` fields this rule asks for as a missing section. simplint stops with
an error when both apply to the same file, so turn the Ruff rule off, or this one.

## Configuration

- [`docstring-style`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#docstring-markup) in
  `[tool.simplint]`: `"rst"` turns the rule on

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def parse(text: str) -> int:
      """Parse a number.

      :param text: The text.
      :returns: The number.
      """
      if not text:
          raise ValueError("empty")
      return int(text)
  ```

## What it does

Checks the docstrings of a library's public API (see [undocumented-public-api](SMP903.html)), when
`docstring-style = "rst"`, for exceptions the function raises itself without a `:raises Error:` field.

Write when the exception can occur. An empty field also triggers [rst-empty-field](SMP914.html).

It also checks functions documented by the project's autodoc directives.

### Active on

- **`raise` statements that name an exception class**: each exception the function raises by name and lets escape needs
  a field. A field matches by the exception's last name, so `:raises errors.ParseError:` documents
  `raise errors.ParseError(...)`.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  import errors


  def parse(text: str) -> int:
      """Parse a number.

      :param text: The text.
      :returns: The number.
      """
      raise errors.ParseError(text)
  ```

### Exceptions

- **A bare `raise`, and a `raise` of an exception the function caught, as `raise error` in `except ... as error`**:
  these re-raise an exception that came from elsewhere rather than naming one.

  - A bare `raise` in an `except` block

    ```toml config
    [project]
    name = "demo"

    [build-system]
    requires = []

    [tool.simplint]
    docstring-style = "rst"
    ```

    ```python ok
    def parse(text: str) -> int:
        """Parse a number.

        :param text: The text.
        :returns: The number.
        """
        try:
            return int(text)
        except ValueError:
            raise
    ```

  - A `raise error` of the exception that `except ValueError as error` caught

    ```toml config
    [project]
    name = "demo"

    [build-system]
    requires = []

    [tool.simplint]
    docstring-style = "rst"
    ```

    ```python ok
    def parse(text: str) -> int:
        """Parse a number.

        :param text: The text.
        :returns: The number.
        """
        try:
            return int(text)
        except ValueError as error:
            raise error
    ```

- **An exception a surrounding `try` of the same function catches**: it does not escape the function. The `try` catches
  it by name, through a base such as [`OSError`](https://docs.python.org/3/builtins/exceptions.html#OSError) for
  [`FileNotFoundError`](https://docs.python.org/3/builtins/exceptions.html#FileNotFoundError), through
  [`Exception`](https://docs.python.org/3/builtins/exceptions.html#Exception), which does not catch
  [`SystemExit`](https://docs.python.org/3/builtins/exceptions.html#SystemExit),
  [`KeyboardInterrupt`](https://docs.python.org/3/builtins/exceptions.html#KeyboardInterrupt) or
  [`GeneratorExit`](https://docs.python.org/3/builtins/exceptions.html#GeneratorExit), or through
  [`BaseException`](https://docs.python.org/3/builtins/exceptions.html#BaseException).

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def read(path: str) -> str:
      """Read a file.

      :param path: Where it is.
      :returns: The text.
      """
      try:
          raise FileNotFoundError(path)
      except OSError:
          return ""
  ```

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def stop(code: int) -> int:
      """Stop the job.

      :param code: The exit code.
      :returns: Nothing.
      """
      try:
          raise SystemExit(code)
      except Exception:
          return 0
  ```

- **An exception whose base a field documents**: a field for the base documents its subclasses too, as
  `:raises OSError:` does `FileNotFoundError` and `:raises ValueError:` does
  [`json.JSONDecodeError`](https://docs.python.org/3/library/json.html#json.JSONDecodeError). A field may name a class
  by a second name, as `:raises IOError:` names `OSError`.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def read(path: str) -> str:
      """Read a file.

      :param path: Where it is.
      :raises OSError: When the file cannot be read.
      :returns: The text.
      """
      raise FileNotFoundError(path)
  ```

- **[`NotImplementedError`](https://docs.python.org/3/builtins/exceptions.html#NotImplementedError)**: it marks a method
  for subclasses to override.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def run() -> None:
      """Run the job."""
      raise NotImplementedError
  ```

- **A `raise` inside a nested function or class**: the rule skips nested definitions, including class bodies that Python
  executes when it defines the class.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def make() -> object:
      """Make a callback.

      :returns: The callback.
      """

      def callback() -> None:
          raise ValueError("later")

      return callback
  ```

- **A name starting in lowercase**: the rule only recognizes a raised name as a class if its first character is
  uppercase.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def run(problem: Exception) -> None:
      """Run the job.

      :param problem: What to raise.
      """
      raise problem
  ```

- **Exceptions raised by the functions it calls**: the code does not show them.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def run() -> None:
      """Run the job."""
      load()
  ```

The rule follows base classes for built-in exceptions, selected standard library exceptions and classes defined in the
file. For other imported exceptions it recognizes matching names and treats `except Exception` as catching them.

## Other tools

- [Ruff DOC501](https://docs.astral.sh/ruff/rules/docstring-missing-exception/) checks explicitly raised exceptions in
  Google and NumPy docstrings. [Ruff PR #13286](https://github.com/astral-sh/ruff/pull/13286) proposes support for
  Sphinx fields.
- [pydoclint DOC501 and DOC503](https://jsh9.github.io/pydoclint/violation_codes.html) cover a missing Raises section
  and mismatched exception names respectively. [darglint DAR401](https://github.com/terrencepreilly/darglint) reports
  missing exception documentation.
