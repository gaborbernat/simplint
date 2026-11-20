Flags a regular expression compiled inside a function, a method or a lambda, such as
[`re.fullmatch(r"...", text)`](https://docs.python.org/3/library/re.html#re.fullmatch), whose pattern a module-level
constant can hold instead, compiled once.

## Why is this bad?

`re.fullmatch(pattern, text)` [looks up the pattern in a cache](https://docs.python.org/3/library/re.html#re.compile)
and compiles it on a miss. A module-level `re.compile()` builds the pattern when the module loads. Subsequent calls to
its `fullmatch()` method skip the lookup, even if other patterns have displaced it from the cache.

A module-level constant also names the pattern, and it is the place where [unexplained-regex](SMP533.html) asks for
comments on its parts.

## Example

```python report
import re


def is_image(image_name: str) -> bool:
    return re.fullmatch(r"[a-z0-9]+(?:/[a-z0-9]+)*", image_name) is not None
```

Use instead:

```python ok
import re
from typing import Final

_IS_IMAGE_PATTERN: Final[re.Pattern[str]] = re.compile(r"[a-z0-9]+(?:/[a-z0-9]+)*")


def is_image(image_name: str) -> bool:
    return _IS_IMAGE_PATTERN.fullmatch(image_name) is not None
```

## What it does

### Active on

- **Calls of `re` functions inside a function body**, where the pattern is a string or bytes literal, including implicit
  concatenation. The functions are `compile`, `match`, `prefixmatch`, `search`, `fullmatch`, `finditer`, `findall`,
  `split`, `sub` and `subn`, called through `re`, an alias of it, or a name imported from it with `from re import`. A
  call in a nested function, a method, a lambda or a comprehension inside one is reported too. The `regex` module counts
  the same way. The rule recognizes integer flags and named flags such as
  [`re.I`](https://docs.python.org/3/library/re.html#re.I), including combinations such as `re.VERBOSE | re.M`.

  **Unsafe fix:** the pattern compiles when the module loads, so an invalid pattern raises at import instead of at the
  call. The fix adds the imports it needs for `Final` and the regex module. The flags expression must also work at
  module level. For example, `re.X | flags` enables verbose mode, so the rule reports it even if `flags` is a local
  parameter. Keep that pattern inside the function when its flags depend on the caller.

  ```python report
  import re


  def find_version(text: str) -> str | None:
      found = re.search(r"v(\d+)\.(\d+)", text, re.IGNORECASE)
      return found.group(0) if found else None
  ```

  ```python fixed
  import re
  from typing import Final

  _FOUND_PATTERN: Final[re.Pattern[str]] = re.compile(r"v(\d+)\.(\d+)", re.IGNORECASE)


  def find_version(text: str) -> str | None:
      found = _FOUND_PATTERN.search(text)
      return found.group(0) if found else None
  ```

  The fix uses the assignment target's name unless it is short or generic, such as `m`, `pattern` or `regex`. In that
  case it uses the enclosing function's name. For example, `version_re = re.compile(...)` becomes `_VERSION_RE`, while
  `pattern = re.compile(...)` in `parse_version` becomes `_PARSE_VERSION_PATTERN`. If the target's name is taken, the
  fix tries the function's name before adding a numeric suffix. For simple local `re.compile()` assignments, the fix can
  remove the assignment and replace its uses with the constant.

  The rule reports a call without a fix when the call contains a comment or sits in a module-level lambda. The fix also
  skips modules with top-level imports after executable statements.

  ```python report
  import re


  def find(text: str) -> str | None:
      found = re.search(
          r"v(\d+)",  # the version
          text,
      )
      return found.group(0) if found else None
  ```

### Exceptions

- **Calls at module level or in a module-level class body** run when the module loads, so nothing is reported.

  ```python ok
  import re

  IS_IMAGE = re.compile(r"[a-z0-9]+(?:/[a-z0-9]+)*")
  ```

- **Plain-text calls covered by Ruff.** [RUF055](https://docs.astral.sh/ruff/rules/unnecessary-regular-expression/)
  suggests string methods for some calls to `re`. The rule skips a `split` with a non-empty pattern and two arguments, a
  `sub` with three arguments whose replacement is a literal or a name bound once to a literal, subject to the escape
  checks in RUF055, and a `match`, `prefixmatch`, `search` or `fullmatch` with two arguments whose result is only tested
  for truth or compared to `None`. Other plain-text calls can still be reported, such as a search whose result is
  assigned to a name, a call with recognized flags, or a call through `regex`.

  ```python ok
  import re


  def has_marker(text: str) -> bool:
      return re.search("marker", text) is not None
  ```

  ```python report
  import re


  def find_marker(text: str) -> str | None:
      found = re.search("marker", text)
      return found.group(0) if found else None
  ```

- **Unknown verbose mode.** When a parameter supplies the flags, the rule usually cannot tell whether verbose mode is
  enabled. It skips these calls.

  ```python ok
  import re


  def find(text: str, flags: int) -> str | None:
      found = re.search(r"v(\d+)", text, flags)
      return found.group(0) if found else None
  ```

## Ruff overlap

Ruff's preview rule
[unnecessary-regular-expression-compile](https://docs.astral.sh/ruff/rules/unnecessary-regular-expression-compile/)
reports a method called on an inline [`re.compile`](https://docs.python.org/3/library/re.html#re.compile), such as
`re.compile(p).match(s)`, and asks for [`re.match(p, s)`](https://docs.python.org/3/library/re.html#re.match) or a
stored pattern. When `p` is a literal inside a function, both rules can report the inline
[`compile()`](https://docs.python.org/3/builtins/functions.html#compile) call. Moving it to a module-level constant
satisfies both.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):**
  [RUF055](https://docs.astral.sh/ruff/rules/unnecessary-regular-expression/) reports the plain-text patterns this rule
  leaves to it. The preview rule named under Ruff overlap reports the inline compile, and this rule fixes it.
