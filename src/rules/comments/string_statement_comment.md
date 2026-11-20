Flags standalone triple-quoted strings used as comments.

## Why is this bad?

Python stores the leading string of a module, class or function in `__doc__`. Documentation tools can also extract
[attribute docstrings](https://peps.python.org/pep-0257/#what-is-a-docstring) after assignments. For an ordinary note
between statements, `#` makes its purpose clear.

## Example

```python report
import math

"""Area of the unit circle."""
AREA = math.pi
```

Use instead:

```python ok
import math

# Area of the unit circle.
AREA = math.pi
```

## What it does

### Active on

- **Standalone triple-quoted strings** are checked in any statement block. A string at the start of an `if` or loop body
  can qualify.

  ```python report
  def run() -> None:
      start()
      """Start the worker before waiting."""
      wait()
  ```

### Exceptions

- **Docstrings**. A string literal at the start of a module, class or function body is its docstring.

  ```python ok
  def run() -> None:
      "Start the worker and wait for it."
      start()
  ```

- **Attribute docstrings**: simplint skips strings immediately after a plain or annotated assignment, including local
  assignments.

  ```python ok
  class Config:
      timeout = 3
      "Seconds to wait for the server."
  ```

- **Single-quoted strings**: a short quoted string often marks a line on purpose, such as a statement a tracer or a test
  expects to run, while triple quotes mark a block of text.

  ```python ok
  def run() -> None:
      start()
      "continued"
      wait()
  ```

### Fix

The fix writes each line of the string as a `#` comment at the statement's indentation, so the code after it keeps its
line numbers. It is safe for the program: a string statement does nothing when it runs. The fix does not apply when:

- every statement in the block is a string
- the string shares its line with other code or a trailing comment, or is an implicit concatenation of several strings
- the text holds a backslash, since the compiler reads escapes and may warn or fail on them
- the module has a `from __future__` import, which the compiler checks against the statements before it
- a line of the text would start a tool directive, such as `# fmt: off`, or start with `!`
- a comment line would run past the comment width, used for the fix

Tools that read the source text, such as
[`inspect.getsource`](https://docs.python.org/3/library/inspect.html#inspect.getsource), see the comments where the
string was.

```python report
def run() -> None:
    start()
    """Start the worker.
    Then wait for it."""
    wait()
```

```python fixed
def run() -> None:
    start()
    # Start the worker.
    # Then wait for it.
    wait()
```

## Ruff overlap

Ruff's useless-expression rule (`B018`) skips string literals, so Ruff does not report these statements.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issue [#11292](https://github.com/astral-sh/ruff/issues/11292).
  Closed pull requests [#11302](https://github.com/astral-sh/ruff/pull/11302) and
  [#27253](https://github.com/astral-sh/ruff/pull/27253) without a rule.
- [pylint](https://github.com/pylint-dev/pylint) reports string statements that have no effect as
  [pointless-string-statement](https://pylint.readthedocs.io/en/latest/user_guide/messages/warning/pointless-string-statement.html),
  with no fix. Its message suggests disabling the check for strings used as documentation, where this rule's fix turns
  the string into `#` comments.
