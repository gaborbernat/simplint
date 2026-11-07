Flags regular expression patterns with two or more unnamed capturing groups, or with one that the code reads by its
number, so the groups are named with `(?P<name>...)`.

## Why is this bad?

A reader of `match.group(1)` and `match.group(2)` counts parentheses to learn which part of the text each group holds. A
named group says what the part is in the pattern, and `match["text"]` says it again where the code reads it. The numbers
also shift when a group is added or moved, which a name survives.

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

Simplint does not invent the names, so it offers no fix for a group the code reads. A group the code never reads has no
name to be read by, and the fix turns it into a non-capturing group instead, as described below.

## What it does

### Active on

- **Patterns of `re` and `regex` calls**, where the pattern is a string or bytes literal, including implicit
  concatenation. The calls are `compile`, `match`, `search`, `fullmatch`, `finditer`, `findall`, `split`, `sub` and
  `subn`, and the methods of a compiled pattern with the same names. The rule counts the unnamed groups of the pattern,
  reading the verbose flag from `re.VERBOSE`, `re.X` or an inline `(?x)`. A `flags` argument that is a name or other
  expression not resolving to a `re` flag leaves the pattern unchecked, since its value may turn verbose mode on.

  - **Two or more unnamed groups** are reported at the pattern.
  - **One unnamed group** is reported when the code reads it by its number: a `group`, `start`, `end` or `span` call
    with a number from 1 on, a subscript with such a number, or a `groups()` call. The tracking follows a name bound
    once in its function, or at module level, to the call's result, to a match of a compiled pattern, or to the loop
    variable of a `finditer` loop. A pattern with a named group is not reported for one unnamed group, since the number
    the code reads may belong to the named group. A group in a `split`, `findall` or `sub` call is left alone, since
    returning the group's text there is the normal use.

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

- **Unsafe fix:** the groups the code never reads become non-capturing, `(` becomes `(?:`. The fix runs only when the
  pattern and its match objects are all tested for truth, compared to `None`, or discarded, with no group read from
  them. A match object that leaves the code that made it, by being returned, stored or passed on, may have its
  `groups()` read somewhere else, and a compiled pattern shared with another module can do the same, so the fix is
  unsafe. A `split` or `findall` call is left unfixed for that reason too: its result holds the groups' text. A name
  bound more than once in its function is not followed, so its pattern keeps its groups.

### Exceptions

- **A single unnamed group that the code does not read by number**, such as a lone group in `split` or a match tested
  for truth.

  ```python ok
  import re


  def words(text):
      return re.split(r"(\s+)", text)
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
