Flags a builtin exception raised with a message that has `%` or `{}` placeholders and further arguments meant to fill
them, such as `raise ValueError("bad %s", value)`.

## Why is this bad?

With `raise ValueError("bad %s", 42)`, Python stores both values in the exception's
[`args`](https://docs.python.org/3/builtins/exceptions.html#BaseException.args). Its message reads `('bad %s', 42)`.
[Logging methods](https://docs.python.org/3/library/logging.html#logging.Logger.debug) accept a format string and
separate values. For a [`ValueError`](https://docs.python.org/3/builtins/exceptions.html#ValueError), format the message
before raising it.

## Example

```python report
def check(value):
    if value < 0:
        raise ValueError("bad %s", value)
```

Use instead:

```python ok
def check(value):
    if value < 0:
        raise ValueError(f"bad {value}")
```

## What it does

### Active on

- **A raised builtin exception whose first argument is a string literal with `%` placeholders or `{}` fields, followed
  by more positional arguments.** **Unsafe fix:** when the placeholders and the arguments match one to one, the message
  is formatted with `%` or [`str.format()`](https://docs.python.org/3/builtins/stdtypes.html#str.format), which changes
  the exception's `args`. Ruff's [f-string](https://docs.astral.sh/ruff/rules/f-string/) (UP032) then turns a
  `str.format()` call into an f-string.

  ```python report
  def check(value):
      if value < 0:
          raise ValueError("bad {}", value)
  ```

  ```python fixed
  def check(value):
      if value < 0:
          raise ValueError(f"bad {value}")
  ```

### Exceptions

- **The project's own exceptions**, whose `__init__` may format the message.

- **[`SyntaxError`](https://docs.python.org/3/builtins/exceptions.html#SyntaxError) and its subclasses, and exception
  groups**: their second argument is the error's location or the grouped exceptions, not part of the message.

- **Named placeholders and numbered brace fields.** A message containing only `%(name)s`, `{name}` or `{0}` placeholders
  is outside the check. The rule also skips calls that unpack positional arguments with `*`.

- **No fix** when the number of placeholders differs from the number of values, the message joins adjacent literals, or
  the edit would remove a comment.

  ```python report
  def check(value, limit):
      if value < 0:
          raise ValueError("bad %s", value, limit)
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):**
  [raw-string-in-exception](https://docs.astral.sh/ruff/rules/raw-string-in-exception/) (EM101) asks to move the literal
  to a variable, which keeps the bug.
- **[pylint](https://github.com/pylint-dev/pylint):**
  [raising-format-tuple](https://pylint.readthedocs.io/en/stable/user_guide/messages/warning/raising-format-tuple.html)
  (W0715).
