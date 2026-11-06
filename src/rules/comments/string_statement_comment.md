Flags triple-quoted string statements used as comments: strings that are neither the docstring of a module, class or
function nor an attribute docstring.

## Why is this bad?

Python evaluates a bare string and throws it away, so it documents nothing the way a docstring does, and no tool reads
it as a comment either. Pylint reports it as a pointless statement. A `#` comment says the same thing where a reader
looks for it.

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

- **Strings between statements**: a triple-quoted string that is not the first statement of its body, and does not
  follow an assignment, is a comment in disguise.

  ```python report
  def run() -> None:
      start()
      """Start the worker before waiting."""
      wait()
  ```

### Exceptions

- **Docstrings**: the first statement of a module, class or function body is its docstring.

  ```python ok
  def run() -> None:
      "Start the worker and wait for it."
      start()
  ```

- **Attribute docstrings**: a string right after an assignment documents the attribute it assigns, as Sphinx reads it.

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

- every statement of its body is a string, since the body would be left empty
- the string shares its line with other code or a trailing comment, or is an implicit concatenation of several strings
- the text holds a backslash, since the compiler reads escapes and may warn or fail on them
- the module has a `from __future__` import, which the compiler checks against the statements before it
- a line of the text would start a tool directive, such as `# fmt: off`, or start with `!`
- a comment line would run past the comment width, which [long-comment-line](SMP007.html) then reports

Tools that read the source text, such as `inspect.getsource`, see the comments where the string was.

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

- [pylint](https://github.com/pylint-dev/pylint) reports string statements that have no effect as
  [pointless-string-statement](https://pylint.readthedocs.io/en/latest/user_guide/messages/warning/pointless-string-statement.html),
  with no fix. Its message suggests disabling the check for strings used as documentation, where this rule's fix turns
  the string into `#` comments.
