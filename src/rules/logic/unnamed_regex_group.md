Flags a regular expression with two or more unnamed captures, or one unnamed capture that the code reads by number.

## Why is this bad?

A reader of `match.group(1)` and `match.group(2)` counts parentheses to learn which part of the text each group holds.
The [regex HOWTO](https://docs.python.org/3/howto/regex.html#non-capturing-and-named-groups) says group numbers become
hard to track in a complex pattern. A name says what the part is where the pattern is written, and `match["text"]` says
it again where the code reads it. Adding or moving a capture can shift existing group numbers. Use a named capture for
text callers need, or a non-capturing group `(?:...)` when callers need only the grouping.

## Example

```python report
import re

LINE = re.compile(r"^(.*?)\s*\{([^}]+)\}\s*$")


def parse(line):
    match = LINE.match(line)
    return match.group(1), match.group(2)
```

Use instead:

```python ok
import re

LINE = re.compile(r"^(?P<text>.*?)\s*\{(?P<tag>[^}]+)\}\s*$")


def parse(line):
    match = LINE.match(line)
    return match["text"], match["tag"]
```

Choose a name for each capture the code reads. Simplint can remove unused captures automatically.

## What it does

### Active on

- **Patterns of `re` and `regex` calls**, where the pattern is a string or bytes literal, including implicit
  concatenation. The rule reads the pattern argument of `compile`, `match`, `prefixmatch`, `search`, `fullmatch`,
  `finditer`, `findall`, `split`, `sub` and `subn`. For a compiled pattern it also follows calls to its methods to find
  out how the captures are used. The rule counts the unnamed groups of the pattern, reading the verbose flag from
  [`re.VERBOSE`](https://docs.python.org/3/library/re.html#re.VERBOSE),
  [`re.X`](https://docs.python.org/3/library/re.html#re.X) or an inline `(?x)`. If simplint cannot resolve the flags, it
  skips the pattern unless the pattern itself enables verbose mode globally.

  - **Two or more unnamed groups** are reported at the pattern, unless the call is `findall` or `split`.
  - **One unnamed group** is reported when the code reads it by its number: a `group`, `start`, `end` or `span` call
    with a number from 1 on, a subscript with such a number, or a `groups()` call. The tracking follows a name bound
    once in its function, or at module level, to the call's result, to a match of a compiled pattern, or to the loop
    variable of a `finditer` loop. A pattern with a named group is not reported for one unnamed group, since the number
    the code reads may belong to the named group. A single capture in `split`, `findall`, `sub` or `subn` is left alone.

  ```python report
  import re


  def is_version(text):
      return bool(re.match(r"(\d+)\.(\d+)", text))
  ```

  ```python fixed
  import re


  def is_version(text):
      return bool(re.match(r"(?:\d+)\.(?:\d+)", text))
  ```

- **Unsafe fix.** Simplint changes unused unnamed captures from `(` to `(?:`. It follows uses such as truth tests,
  comparisons to `None` and reads of the whole match. For `sub` and `subn`, the replacement must be a literal without
  backslashes. A backreference in the pattern, an untracked use of the match, or a string escape that changes the source
  offsets prevents the fix. Removing captures changes the compiled pattern's group count.

  Simplint also withholds the fix if the pattern is bound in a class body, its name has multiple bindings, or code
  outside the scope may read the name. Examples follow.

  - **A pattern in a class body**: reported, with no fix.

    ```python report
    import re


    class Parser:
        LINE = re.compile(r"(\d+)-(\d+)")
    ```

    ```python fixed
    import re


    class Parser:
        LINE = re.compile(r"(\d+)-(\d+)")
    ```

  - **A name a function declares `global`**: a pattern bound to it, in the function or at module level, is reported,
    with no fix.

    ```python report
    import re

    PATTERN = re.compile(r"(\d+)-(\d+)")


    def reset():
        global PATTERN
        PATTERN = re.compile(r"(\d+)-(\d+)")
    ```

    ```python fixed
    import re

    PATTERN = re.compile(r"(\d+)-(\d+)")


    def reset():
        global PATTERN
        PATTERN = re.compile(r"(\d+)-(\d+)")
    ```

  - **A name a function declares `nonlocal`**: the pattern bound to it is reported, with no fix.

    ```python report
    import re


    def outer():
        pattern = None

        def reset():
            nonlocal pattern
            pattern = re.compile(r"(\d+)-(\d+)")

        reset()
        return pattern
    ```

    ```python fixed
    import re


    def outer():
        pattern = None

        def reset():
            nonlocal pattern
            pattern = re.compile(r"(\d+)-(\d+)")

        reset()
        return pattern
    ```

  - **A module name another module imports**: reported, with no fix.

    ```python report file=example.py
    import re

    PATTERN = re.compile(r"(\d+)-(\d+)")
    ```

    ```python fixed file=example.py
    import re

    PATTERN = re.compile(r"(\d+)-(\d+)")
    ```

    ```python context file=main.py
    from example import PATTERN
    ```

  - **A module name `__all__` lists**: reported, with no fix.

    ```python report
    import re

    __all__ = ["PATTERN"]
    PATTERN = re.compile(r"(\d+)-(\d+)")
    ```

    ```python fixed
    import re

    __all__ = ["PATTERN"]
    PATTERN = re.compile(r"(\d+)-(\d+)")
    ```

### Exceptions

- **A single unnamed group that the code does not read by number**, such as a lone group in `split` or a match tested
  for truth.

  ```python ok
  import re


  def words(text):
      return re.split(r"(\s+)", text)
  ```

- **Groups of a `findall` or `split` call**, or of a compiled pattern only called with those: the code receives the
  groups as tuple members or list items by position, so a name would not reach it.

  ```python ok
  import re


  def natural_key(value):
      return [(text, int(digits or 0)) for text, digits in re.findall(r"(\D*)(\d*)", value)]
  ```

- **Named groups** are what the rule asks for.

  ```python ok
  import re


  def is_version(text):
      return bool(re.match(r"(?P<major>\d+)\.(?P<minor>\d+)", text))
  ```

- **Non-capturing groups, lookarounds and inline flags** do not capture, so they are not counted.

  ```python ok
  import re


  def is_version(text):
      return bool(re.match(r"(?:\d+)\.(?=\d)(?i:\d+)", text))
  ```

- **A parenthesis in a character class or escaped with a backslash** is text, not a group.

  ```python ok
  import re


  def is_call(text):
      return bool(re.match(r"[(]\(\d+\)", text))
  ```

- **A parenthesis in a verbose comment** is ignored when the verbose flag is on.

  ```python ok
  import re


  def is_version(text):
      return bool(
          re.match(
              r"""\d+  # the major (or the minor) number
          \.\d+""",
              text,
              re.VERBOSE,
          )
      )
  ```

- **A parenthesis in a verbose comment of a scoped group**: `(?x:...)` turns verbose mode on for its own group only, so
  its `#` comment hides the parenthesis, and the mode goes back after the group's `)`. A `(?-x:...)` group turns it off,
  so there its `#` is text and its parentheses count.

  ```python ok
  import re


  def is_version(text):
      return re.match(
          r"""(?x:(\d+)  # the major (or the minor) number
      )""",
          text,
      )
  ```

  ```python report
  import re


  def pair(text):
      return re.match(r"(?-x:(a) # (b))", text, re.VERBOSE)
  ```

- **A pattern that is not a literal**, such as an f-string or a name, is not read.

  ```python ok
  import re


  def is_version(text, prefix):
      return bool(re.match(f"{prefix}(\\d+)\\.(\\d+)", text))
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** no rule reports unnamed capture groups. Open issue
  [#4935](https://github.com/astral-sh/ruff/issues/4935) asks for a port of Sonar's Python checks in general, which does
  not name this one.
