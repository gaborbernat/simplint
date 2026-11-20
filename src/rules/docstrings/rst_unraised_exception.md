Flags documented exceptions that simplint cannot trace in a public API function.

## Why is this bad?

After a function stops raising an exception, its old `:raises:` field can lead callers to write a handler they no longer
need. Review the field before removing it. A user-defined method such as `__getitem__` can raise exceptions the rule
cannot infer.

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
`docstring-style = "rst"`, for `:raises Error:` fields without a matching source of that exception.
[rst-missing-raises](SMP924.html) checks the reverse case.

The unsafe fix removes the field and its continuation lines. Check that calls or overloaded operators cannot raise the
documented exception. Removing a field also changes `__doc__`. The field must start below the opening quotes in a single
triple-quoted string whose text matches the source. If the field ends the docstring, a comment after the closing quotes
also prevents the fix.

It also checks functions documented by the project's autodoc directives.

### Active on

- **Exceptions the rule can trace.** It checks `raise` statements, follows calls to undecorated local functions and
  methods of the same class, and accounts for selected built-in exceptions from operations listed below. A field matches
  by the exception's last name or a second name, such as
  [`IOError`](https://docs.python.org/3/builtins/exceptions.html#IOError) for
  [`OSError`](https://docs.python.org/3/builtins/exceptions.html#OSError), and a field for a base, such as
  `:raises OSError:`, matches a raised subclass such as
  [`FileNotFoundError`](https://docs.python.org/3/builtins/exceptions.html#FileNotFoundError). The bases of built-in
  exceptions, of common standard library exceptions and of the classes the file defines are known.

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

- **Code the rule cannot trace.** It skips the whole function for an unresolved call, a decorated callee, a generator
  expression, `with`, `await`, `yield from`, or a `raise` whose class it cannot identify. A loop or comprehension over
  anything except a list, tuple or set display also triggers this exception. Calls named `isinstance`, `issubclass`,
  `len`, `repr`, `id`, `type`, `callable`, `hasattr` and `print` do not trigger it. The rule does not trace their
  implementations.

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

- **Built-in exceptions associated with operations.** The rule accepts the following fields when it sees the
  corresponding operation.

  | Operation                                     | Documented exceptions                                                                                                                                                                                                                                                                                                  |
  | --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
  | Subscript                                     | [`KeyError`](https://docs.python.org/3/builtins/exceptions.html#KeyError), [`IndexError`](https://docs.python.org/3/builtins/exceptions.html#IndexError), [`LookupError`](https://docs.python.org/3/builtins/exceptions.html#LookupError), [`TypeError`](https://docs.python.org/3/builtins/exceptions.html#TypeError) |
  | Attribute access                              | [`AttributeError`](https://docs.python.org/3/builtins/exceptions.html#AttributeError)                                                                                                                                                                                                                                  |
  | Binary operator, unary operator or comparison | [`ZeroDivisionError`](https://docs.python.org/3/builtins/exceptions.html#ZeroDivisionError), [`ArithmeticError`](https://docs.python.org/3/builtins/exceptions.html#ArithmeticError), [`OverflowError`](https://docs.python.org/3/builtins/exceptions.html#OverflowError), `TypeError`                                 |
  | `assert`                                      | [`AssertionError`](https://docs.python.org/3/builtins/exceptions.html#AssertionError)                                                                                                                                                                                                                                  |

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

- **`:raises Exception:` and `:raises BaseException:`**: the rule always accepts these broad fields.

## Other tools

- [Ruff DOC502](https://docs.astral.sh/ruff/rules/docstring-extraneous-exception/) checks exception sections in Google
  and NumPy docstrings.
- [pydoclint DOC502 and DOC503](https://jsh9.github.io/pydoclint/violation_codes.html) cover a Raises section without a
  raise statement and mismatched exception names respectively.
  [darglint DAR402](https://github.com/terrencepreilly/darglint) reports extraneous exception documentation.
