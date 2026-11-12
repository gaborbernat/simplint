Flags a regular expression compiled inside a function, a method or a lambda, such as `re.fullmatch(r"...", text)`, whose
pattern a module-level constant can hold instead, compiled once.

## Why is this bad?

A module-level `re` function such as `re.fullmatch(pattern, text)` skips compiling its pattern only while the pattern is
in `re`'s cache. The call runs
[`re._compile`](https://github.com/python/cpython/blob/v3.16.0a2/Lib/re/__init__.py#L332-L373), which builds the key
`(type(pattern), pattern, flags)` and looks it up in `_cache2`
([line 337](https://github.com/python/cpython/blob/v3.16.0a2/Lib/re/__init__.py#L337)) and then in `_cache`
([line 343](https://github.com/python/cpython/blob/v3.16.0a2/Lib/re/__init__.py#L343)). The two caches hold 256 and 512
patterns ([lines 328 and 329](https://github.com/python/cpython/blob/v3.16.0a2/Lib/re/__init__.py#L328-L329)), so a
program that uses more patterns than that compiles some of them again on every call. A module-level constant is compiled
once, when the module loads, and each call skips the lookup.

Measured with `timeit` on CPython 3.15.2, which has the same cache code as the cited tag, two runs gave the same
picture. On a warm cache, `re.fullmatch(p, s)` took 231 ns and 238 ns against 105 ns and 109 ns for `_P.fullmatch(s)`,
about 2.2 times as long. With 600 patterns cycled, which evicts each one before its next use, every one of the 60,000
inline calls compiled its pattern and took 21.3 µs and 21.9 µs, against 68 ns and 69 ns for precompiled patterns, about
310 to 320 times as long.

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
  the same way. The flags, if any, must be constant, as `re.I`, `re.VERBOSE | re.M` or `flags=re.I`.

  **Unsafe fix:** the pattern compiles when the module loads, so an invalid pattern raises at import instead of at the
  call. The fix adds the `Final` and `re` imports the file lacks.

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

  The constant's name comes from the name the call's result is assigned to when that name has three or more letters, and
  from the function otherwise, in upper case. `_PATTERN` follows unless the last word of the name already says pattern
  or regex, so `pattern = re.compile(...)` gives `_PATTERN`, `version_re = re.compile(...)` gives `_VERSION_RE`, and
  `m = re.match(...)` in `parse_version` gives `_PARSE_VERSION_PATTERN`. A name the file already spells gets a number
  after it.

  A call with a comment inside it is reported without a fix, since the rewritten call would drop the comment.

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

- **Calls at module level or in a class body** run when the module loads, so nothing is reported.

  ```python ok
  import re

  IS_IMAGE = re.compile(r"[a-z0-9]+(?:/[a-z0-9]+)*")
  ```

- **Plain-text calls that Ruff rewrites**: a pattern with no regular expression metacharacter, called through `re`, is
  left to [RUF055](https://docs.astral.sh/ruff/rules/unnecessary-regular-expression/) when Ruff rewrites the call to a
  `str` method. Ruff rewrites a `split` with a non-empty pattern and two arguments, a `sub` with three arguments whose
  replacement is a literal without an escape that `re` rejects, and a `match`, `prefixmatch`, `search` or `fullmatch`
  with two arguments whose result is only tested for truth or compared to `None`. A module constant would hold such a
  pattern without a reason, so these calls stay with Ruff. Any other plain-text call is reported, such as a result
  stored in a name, a `re` call with flags or a third argument, or a pattern passed to `regex`.

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

- **Flags that are not constant**, such as a parameter, are not reported, since the flags decide what the constant
  compiles to.

  ```python ok
  import re


  def find(text: str, flags: int) -> str | None:
      found = re.search(r"v(\d+)", text, flags)
      return found.group(0) if found else None
  ```

## Conflicts with Ruff

Ruff's preview rule
[unnecessary-regular-expression-compile](https://docs.astral.sh/ruff/rules/unnecessary-regular-expression-compile/)
reports an inline `re.compile(p).match(s)` used once and suggests `re.match(p, s)`. That advice only holds for code that
runs once. Inside a function or method that gets called repeatedly, and inside a loop, both forms pay the compile or the
cache lookup on every call. The measurements under Why is this bad? put an inline call at about 2.2 times a precompiled
one on a warm cache, and at about 310 to 320 times once the cache is evicted. Only a module-level constant avoids that.

The two rules disagree inside functions only, since calls at module level are not reported here. Both report the same
inline `re.compile`, and the Ruff rule has no fix, so simplint and Ruff work against each other while both are on, and
the run exits with status 2. Ignore the Ruff rule in `lint.ignore`, the recommended way, or ignore this rule in
`[tool.simplint]`.
[Resolving conflicts with Ruff](https://simplint.readthedocs.io/en/latest/how-to/resolve-ruff-conflicts.html#resolve-ruff-conflicts)
lists the pair.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):**
  [RUF055](https://docs.astral.sh/ruff/rules/unnecessary-regular-expression/) reports the plain-text patterns this rule
  leaves to it. The preview rule named under Conflicts with Ruff reports the inline compile.
