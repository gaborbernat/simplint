Flags [`re.match()`](https://docs.python.org/3/library/re.html#re.match) and
[`re.prefixmatch()`](https://docs.python.org/3/library/re.html#re.prefixmatch) with a pattern anchored at its end, and
[`re.search()`](https://docs.python.org/3/library/re.html#re.search) with a pattern anchored at both ends.
[`re.fullmatch()`](https://docs.python.org/3/library/re.html#re.fullmatch) says the whole string must match.

## Why is this bad?

A trailing `$` matches at the end of the string [or just before](https://docs.python.org/3/library/re.html#index-2) a
newline that ends it, so `re.match(r"\d+$", text)` accepts `"123\n"`. Validation built this way lets a trailing newline
through to whatever uses the value next. `re.fullmatch()` matches the whole string only, and its name states the intent
the anchors spell out.

## Example

```python report
import re


def is_number(text):
    return re.match(r"\d+$", text) is not None
```

Use instead:

```python ok
import re


def is_number(text):
    return re.fullmatch(r"\d+", text) is not None
```

## What it does

### Active on

- **`re.match()` and `re.prefixmatch()` with a literal pattern that ends in `$` or `\Z`**, and `re.search()` with one
  that also starts with `^` or `\A`. **Fix:** the call becomes `re.fullmatch()` and the anchors are deleted. It is safe
  for `\Z`, which matches only at the end, and unsafe for `$`, since `re.fullmatch()` rejects the trailing newline that
  `$` accepts.

  ```python report
  import re


  def is_number(text):
      return re.search(r"^\d+$", text) is not None
  ```

  ```python fixed
  import re


  def is_number(text):
      return re.fullmatch(r"\d+", text) is not None
  ```

- **Names imported from `re`**, such as `from re import match`, reported without a fix, since renaming the name would
  rename its other uses too.

  ```python report
  from re import match


  def is_number(text):
      return match(r"\d+\Z", text) is not None
  ```

### Exceptions

- **Patterns with a top-level `|`**: `a|b$` anchors only its last branch.

  ```python ok
  import re


  def is_word(text):
      return re.match(r"yes|no$", text) is not None
  ```

- **Flags, as an argument or an inline group such as `(?m)`**: with `re.MULTILINE`, `$` matches at the end of each line.

  ```python ok
  import re


  def has_number_line(text):
      return re.match(r"\d+$", text, re.MULTILINE) is not None
  ```

- **`re.search()` without a start anchor**, which finds the pattern anywhere.

  ```python ok
  import re


  def ends_in_number(text):
      return re.search(r"\d+$", text) is not None
  ```

- **Compiled patterns**: the receiver of `.match()` may be any object.

  ```python ok
  import re

  NUMBER = re.compile(r"\d+$")


  def is_number(text):
      return NUMBER.match(text) is not None
  ```
