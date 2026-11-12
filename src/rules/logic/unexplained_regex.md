Flags a regular expression pattern of four or more parts that has no comment, a `#` comment in verbose mode or a
`(?#...)` comment. A part is a group, an alternation, a character class with its repeat, or a repeat of a group.

## Why is this bad?

A reader of a pattern works out each part from its punctuation, and a long pattern holds many parts to work out. The
[regex HOWTO](https://docs.python.org/3/howto/regex.html#using-re-verbose) says compact patterns are not very readable
and recommends [`re.VERBOSE`](https://docs.python.org/3/library/re.html#re.VERBOSE) with a `#` comment on each part. A
comment on the part's own line stays with that part, where a note above the pattern drifts away from it. The verbose
flag alone says nothing about the parts, so the pattern still needs a comment to be read.

## Example

```python report
import re


def parse_date(text):
    return re.match(r"^(\d{4})-(\d{2})-(\d{2})(?:T[0-2]\d:[0-5]\d)?$", text)
```

Use instead:

```python ok
import re


def parse_date(text):
    return re.match(
        r"""
        ^
        (\d{4})  # the year
        -
        (\d{2})  # the month
        -
        (\d{2})  # the day
        (?:T[0-2]\d:[0-5]\d)?  # an optional time
        $
        """,
        text,
        re.VERBOSE,
    )
```

## What it does

### Active on

- **Patterns of `re` and `regex` calls**, where the pattern is a string or bytes literal, including implicit
  concatenation. The calls are the same as in [unnamed-regex-group](SMP532.html): the module functions and the methods
  of a compiled pattern. The rule counts the parts of the pattern, reading the verbose flag from
  [`re.VERBOSE`](https://docs.python.org/3/library/re.html#re.VERBOSE),
  [`re.X`](https://docs.python.org/3/library/re.html#re.X) or an inline `(?x)`. A `flags` argument that is a name or
  other expression not resolving to a `re` flag leaves the pattern unchecked, since its value may turn verbose mode on.

  A part is:

  - each group, of any kind: capturing, named, non-capturing, lookaround, atomic and conditional groups. An inline flag
    group such as `(?i:...)` is a group, while a flag set for the whole pattern, such as `(?i)`, is not
  - each `|` outside a character class
  - each character class `[...]`, which counts once with the repeat after it, such as `[...]{2,3}` or `[ \t]*`
  - each repeat of a group, such as `(...)*` or `(...)+?`, which counts on top of the group

  Escapes and literal characters are not parts, and neither is a reference to a group by name or number.

  - **Four or more parts with no comment** are reported at the pattern. A pattern without `re.VERBOSE` is told to use
    the flag and a `#` comment on each part.
  - **Four or more parts in a verbose pattern with no `#` comment** outside a character class are reported at the
    pattern too, since the flag alone explains no part.

  A `(?#...)` comment explains a pattern whether or not it is verbose, so a pattern with one is not reported.

  ```python report
  import re


  def is_version(text):
      return bool(re.match(r"^(\d+)\.(\d+)(?:\.(\d+))?$", text, re.VERBOSE))
  ```

### Exceptions

- **A short pattern** with fewer than four parts reads at a glance, and a class with its repeat is one part of it.

  ```python ok
  import re


  def is_version(text):
      return bool(re.match(r"(\d+)\.(\d+)\.(\d+)", text))
  ```

  ```python ok
  import re


  def split_items(text):
      return re.split(r"[ \t]*,[ \t]*", text)
  ```

- **A pattern with a `(?#...)` comment** is explained, as a `#` comment in verbose mode explains it.

  ```python ok
  import re


  def is_version(text):
      return bool(re.match(r"(\d+)\.(\d+)(?:\.(\d+))?(?#the optional patch)$", text))
  ```

- **A verbose pattern with a `#` comment** explains its parts, whatever their number.

  ```python ok
  import re


  def is_version(text):
      return bool(
          re.match(
              r"""
              (\d+)  # the major version
              \.
              (\d+)  # the minor version
              """,
              text,
              re.VERBOSE,
          )
      )
  ```

- **A `#` comment in a scoped `(?x:...)` group** explains the pattern, as a comment in a verbose pattern does.

  ```python ok
  import re


  def is_version(text):
      return re.match(
          r"""(?x:(\d+)  # the major version
      )\.(\d+)\.(\d+)\.(\d+)""",
          text,
      )
  ```

- **A pattern that is not a literal**, such as an f-string or a name, is not read.

  ```python ok
  import re


  def is_version(prefix, text):
      return re.match(rf"{prefix}(\d+)\.(\d+)\.(\d+)\.(\d+)", text)
  ```

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** no rule asks for comments on a regular expression. Ruff's
  [FURB167](https://docs.astral.sh/ruff/rules/regex-flag-alias/) asks for the flag names of `re` instead of their
  aliases.
