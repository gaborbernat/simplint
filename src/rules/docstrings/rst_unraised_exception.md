Flags `:raises Error:` fields in a public API function's reStructuredText docstring for an exception the function never
raises.

## Why is this bad?

Callers write handlers for the exceptions a docstring promises. A field for an exception the code no longer raises sends
them to handle a failure that cannot happen and hides which ones can. The field usually outlives a refactor that moved
or dropped the `raise`.

## Example

```toml config
[tool.simplint]
docstring-style = "rst"
project-kind = "library"
```

```python report
def parse(text: str) -> str:
    """Parse a name.

    :param text: The text.
    :returns: The name.
    :raises ValueError: When the text is empty.
    """
    return text + "!"
```

Use instead:

```python ok
def parse(text: str) -> str:
    """Parse a name.

    :param text: The text.
    :returns: The name.
    """
    return text + "!"
```

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
  def parse(text: str) -> str:
      """Parse a name.

      :param text: The text.
      :returns: The name.
      :raises ValueError: When the text is empty.
      """
      return text + "!"
  ```

## What it does

Checks the docstrings of a library's public API (see [undocumented-public-api](SMP903.html)), when
`docstring-style = "rst"`, for `:raises Error:` fields whose exception the function cannot raise. It is the other side
of [rst-missing-raises](SMP924.html), which asks for the fields a `raise` needs.

The fix removes the field. It is offered when the field's lines can be deleted on their own.

### Active on

- **Bodies whose every source of exceptions is known**: the function's `raise` statements, the module functions and
  methods of the same class it calls by name, and operations that raise a built-in exception on their own. A field
  matches by the exception's last name or a second name, such as `IOError` for `OSError`, and a field for a base, such
  as `:raises OSError:`, matches a raised subclass such as `FileNotFoundError`. The bases of built-in exceptions, of
  common standard library exceptions and of the classes the file defines are known.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python report
  def check(text: str) -> str:
      """Check a name.

      :param text: The text.
      :returns: The text.
      :raises KeyError: When the name is unknown.
      """
      return _validated(text)


  def _validated(text: str) -> str:
      if not text:
          raise ValueError(text)
      return text
  ```

### Exceptions

- **Anything that can raise what the code does not show**: a call of a function this module does not define, a loop or
  comprehension over anything but a literal display, a `with`, an `await`, a `yield from`, or a `raise` of a value whose
  class is unknown. The rule then judges none of the function's fields.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  import json


  def load(text: str) -> object:
      """Load a document.

      :param text: The text.
      :returns: The document.
      :raises ValueError: When the text is not JSON.
      """
      return json.loads(text)
  ```

- **Exceptions an operation raises on its own**: a subscript can raise `KeyError`, `IndexError`, `LookupError` or
  `TypeError`, an attribute read `AttributeError`, an arithmetic operator or comparison `ZeroDivisionError`,
  `ArithmeticError`, `OverflowError` or `TypeError`, and an `assert` `AssertionError`.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def first(names: dict[str, str]) -> str:
      """Return the first name.

      :param names: The names.
      :returns: The name.
      :raises KeyError: When there is none.
      """
      return names["first"]
  ```

- **A bare `raise` or a `raise` of the caught exception**: it re-raises whatever the surrounding handler catches, so
  each class the handler names counts as raised.

  ```toml config
  [project]
  name = "demo"

  [build-system]
  requires = []

  [tool.simplint]
  docstring-style = "rst"
  ```

  ```python ok
  def parse(text: str) -> str:
      """Parse a name.

      :param text: The text.
      :returns: The name.
      :raises ValueError: When the text is empty.
      """
      try:
          return _validated(text)
      except ValueError:
          raise


  def _validated(text: str) -> str:
      if not text:
          raise ValueError(text)
      return text
  ```

- **`:raises Exception:` and `:raises BaseException:`**: they document any failure.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):**
  [DOC502](https://docs.astral.sh/ruff/rules/docstring-extraneous-exception/) reports the same case in Google and
  numpydoc sections, and cannot read `:raises:` fields.
- [pydoclint](https://github.com/jsh9/pydoclint) reports documented exceptions the body does not raise as
  [DOC502 and DOC503](https://jsh9.github.io/pydoclint/violation_codes.html), with no fix.
- [darglint](https://github.com/terrencepreilly/darglint) reports them as
  [DAR402](https://github.com/terrencepreilly/darglint), with no fix.
