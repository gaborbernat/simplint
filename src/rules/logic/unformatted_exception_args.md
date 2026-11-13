Flags a builtin exception raised with a message that has `%` or `{}` placeholders and further arguments meant to fill
them, such as `raise ValueError("bad %s", value)`.

## Why is this bad?

An exception does not format its message: it keeps every positional argument in
[`args`](https://docs.python.org/3/library/exceptions.html#BaseException.args), and `str()` of an exception with more
than one shows the tuple. `raise ValueError("bad %s", value)` reports `('bad %s', 42)`. The
[logging calls](https://docs.python.org/3/library/logging.html#logging.Logger.debug) take arguments this way, which is
where the habit comes from.

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
  is formatted with `%` or `str.format()`, which changes the exception's `args`. Ruff's
  [f-string](https://docs.astral.sh/ruff/rules/f-string/) (UP032) then turns a `str.format()` call into an f-string.

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

- **[`SyntaxError`](https://docs.python.org/3/library/exceptions.html#SyntaxError) and its subclasses, and exception
  groups**: their second argument is the error's location or the grouped exceptions, not part of the message.

- **No fix** for named placeholders such as `%(name)s` or `{name}`, a count that differs from the arguments, or an
  implicitly concatenated message.

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
