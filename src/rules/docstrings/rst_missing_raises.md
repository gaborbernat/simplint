Flags exceptions a public API function raises itself that its reStructuredText docstring leaves without a
`:raises Error:` field.

## Why is this bad?

Callers handle the exceptions they know about, and no signature names them, so the docstring is where they learn them.
[PEP 257](https://peps.python.org/pep-0257/#multi-line-docstrings) lists the exceptions raised among what a function's
docstring documents, and the
[Google style guide](https://google.github.io/styleguide/pyguide.html#383-functions-and-methods) asks for those relevant
to the interface. Google says not to document exceptions raised when a caller misuses the API; simplint reports them
anyway, since an undocumented one reaches callers as a surprise, often in production, where the error path is first
exercised.

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

No fix is offered: the description is for the author to write, and [rst-empty-field](SMP914.html) reports the empty
field until they do.

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
  it by name, through a built-in base such as [`OSError`](https://docs.python.org/3/builtins/exceptions.html#OSError)
  for [`FileNotFoundError`](https://docs.python.org/3/builtins/exceptions.html#FileNotFoundError), through
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

- **An exception whose built-in base a field documents**: a field for the base documents its subclasses too, as
  `:raises OSError:` does `FileNotFoundError`.

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

- **A `raise` inside a nested function or class**: it runs when the nested function or class does, not as part of this
  function's body.

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

- **A name starting in lowercase**: it is a variable rather than an exception class.

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

The bases of built-in exceptions are known; those of a project's own exceptions are not.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of preview rule
  [DOC501](https://docs.astral.sh/ruff/rules/docstring-missing-exception/), which cannot read `:raises:` fields, so
  simplint refuses to run with it on reST docstrings; open pull request
  [#13286](https://github.com/astral-sh/ruff/pull/13286) adds Sphinx support.
- [pydoclint](https://github.com/jsh9/pydoclint) reports raised exceptions missing from or extra in the docstring as
  [DOC501](https://jsh9.github.io/pydoclint/violation_codes.html) and
  [DOC502](https://jsh9.github.io/pydoclint/violation_codes.html), with no fix.
- [darglint](https://github.com/terrencepreilly/darglint) reports the same exceptions as
  [DAR401](https://github.com/terrencepreilly/darglint) and [DAR402](https://github.com/terrencepreilly/darglint), with
  no fix.
