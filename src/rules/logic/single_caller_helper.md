Flags a private module-level function that only one other function calls, when inlining its statements into that caller
keeps the caller short.

## Why is this bad?

A helper with a single caller splits one piece of work over two functions. A reader has to jump to the helper to learn
what the caller does, and the helper's name adds little when only one place uses it.
[John Ousterhout](https://github.com/johnousterhout/aposd-vs-clean-code#method-length) argues that many short functions
are harder to read: each split adds an interface to learn while hiding little, and functions that only make sense
together read better combined. Inlining the helper's statements keeps the work in one function, as long as the caller
stays short enough to read at once. [Martin Fowler](https://martinfowler.com/bliki/FunctionLength.html) and
[Robert Martin](https://github.com/johnousterhout/aposd-vs-clean-code#method-length) favor small functions named by
intent, which is why the rule skips a caller that inlining would make long.

## Example

```python report
def load(text: str) -> dict[str, str]:
    header = _parse_header(text)
    return {"header": header}


def _parse_header(text: str) -> str:
    first = text.splitlines()[0]
    return first.strip()
```

Use instead:

```python ok
def load(text: str) -> dict[str, str]:
    first = text.splitlines()[0]
    header = first.strip()
    return {"header": header}
```

## Configuration

- [`logic.max-helper-statements`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#logic) in
  `[tool.simplint]`: the most statements a helper may have to be suggested for inlining, such as
  `max-helper-statements = 2` under `[tool.simplint.logic]`

  ```toml config
  [tool.simplint.logic]
  max-helper-statements = 2
  ```

  ```python ok
  def load(text: str) -> str:
      return _first(text)


  def _first(text: str) -> str:
      first = text.splitlines()[0]
      return first.strip()
  ```

- [`logic.max-caller-statements`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#logic) in
  `[tool.simplint]`: the most statements the caller may have once the helper's are added, such as
  `max-caller-statements = 30` under `[tool.simplint.logic]`

  ```toml config
  [tool.simplint.logic]
  max-caller-statements = 3
  ```

  ```python ok
  def load(text: str) -> dict[str, str]:
      header = _parse_header(text)
      return {"header": header}


  def _parse_header(text: str) -> str:
      first = text.splitlines()[0]
      return first.strip()
  ```

## What it does

Statements are counted as the top-level statements of a body, each nested block counting as one, and a docstring does
not count. A helper of `max-helper-statements` or more statements is not reported. The caller's statements plus the
helper's must stay within `max-caller-statements` for the helper to be reported. The caller's cognitive complexity must
also stay within `max-cognitive-complexity` once the helper's body is scored at the depth of its call site, so that
[cognitive-complexity](SMP529.html) does not ask to split the caller again.

The caller must also stay within the limits of the Ruff rules that run, which count the inlined statements:
`lint.mccabe.max-complexity` for C901, and `lint.pylint.max-branches`, `lint.pylint.max-statements` and
`lint.pylint.max-returns` for PLR0912, PLR0915 and PLR0911. It also checks PLR1702's nested-block limit, PLW0717's
try-clause statement limit and TRY301 when a helper raises inside a caller's `try` block. These counts are conservative
estimates, so a caller they might report is left as it is.

### Active on

- **Private module-level functions with one call site in the run**: a function whose name starts with an underscore,
  that no other module imports and no string names, and that one other function calls. The call is the only reference to
  the helper, and the helper has fewer statements than `max-helper-statements`.

  ```python report
  def load(text: str) -> dict[str, str]:
      header = _parse_header(text)
      return {"header": header}


  def _parse_header(text: str) -> str:
      first = text.splitlines()[0]
      return first.strip()
  ```

- **A call whose `(` starts a later line**: the call counts like one on a single line, so its helper is reported when
  that is the only call.

  ```python report
  def load(name):
      return [
          _fill_a(name)  # why
      ]


  def _fill_a(name):
      value = name.strip()
      return value.upper()
  ```

### Exceptions

- **Helpers that only forward their arguments**: [forwarding-function](SMP501.html) reports them, asking to call the
  target directly rather than inline the wrapper.

  ```python ok
  import json


  def run(path: str) -> dict[str, str]:
      return _load(path)


  def _load(path: str) -> dict[str, str]:
      return json.loads(path)
  ```

- **Helpers called from another module**: inlining would move code across files, so an import of the helper keeps it.

  ```python ok file=helpers.py
  def _parse_header(text: str) -> str:
      first = text.splitlines()[0]
      return first.strip()
  ```

  ```python context file=main.py
  from helpers import _parse_header


  def load(text: str) -> str:
      return _parse_header(text)
  ```

- **Helpers passed as a value**: [`map(_parse_header, texts)`](https://docs.python.org/3/builtins/functions.html#map)
  does not call the helper where it stands.

  ```python ok
  def load(texts: list[str]) -> list[str]:
      return list(map(_parse_header, texts))


  def _parse_header(text: str) -> str:
      first = text.splitlines()[0]
      return first.strip()
  ```

- **A reference that ends its line**: a line that starts with `(` begins a new statement, so the helper is only passed
  on there, not called.

  ```python ok
  def register(table: dict[str, str], key: str) -> None:
      table[key] = _build
      (table)


  def _build(key: str) -> str:
      return key.strip()
  ```

- **Decorated helpers**: a decorator wraps the helper, so the call does not run its body.

  ```python ok
  import functools


  def load(text: str) -> str:
      return _parse_header(text)


  @functools.cache
  def _parse_header(text: str) -> str:
      first = text.splitlines()[0]
      return first.strip()
  ```

- **Helpers named in `__all__`**: the module exports them, so they stay.

  ```python ok
  __all__ = ["_parse_header", "load"]


  def load(text: str) -> str:
      return _parse_header(text)


  def _parse_header(text: str) -> str:
      first = text.splitlines()[0]
      return first.strip()
  ```

- **Recursive helpers**: a helper that calls itself keeps its own call.

  ```python ok
  def load(depth: int) -> int:
      return _count(depth)


  def _count(depth: int) -> int:
      return 0 if depth == 0 else _count(depth - 1)
  ```

- **Calls inside a loop, a comprehension or a lambda**: inlining would run the helper's statements once per pass.

  ```python ok
  def load(texts: list[str]) -> list[str]:
      headers = []
      for text in texts:
          headers.append(_parse_header(text))
      return headers


  def _parse_header(text: str) -> str:
      first = text.splitlines()[0]
      return first.strip()
  ```

- **Helpers with `max-helper-statements` or more statements**: a helper that long stays a named step.

  ```python ok
  def load(text: str) -> str:
      return _parse_header(text)


  def _parse_header(text: str) -> str:
      first = text.splitlines()[0]
      stripped = first.strip()
      return stripped.lower()
  ```

- **A caller that would grow past `max-caller-statements`**: the inlined caller would be too long to read at once.

  ```toml config
  [tool.simplint.logic]
  max-caller-statements = 3
  ```

  ```python ok
  def load(text: str) -> dict[str, str]:
      header = _parse_header(text)
      return {"header": header}


  def _parse_header(text: str) -> str:
      first = text.splitlines()[0]
      return first.strip()
  ```

- **A caller that the inlined helper would push over the cognitive-complexity limit**: a call deep inside the caller
  scores the helper's branches deeper, so the caller would be reported by cognitive-complexity right after inlining.

  ```toml config
  [tool.simplint.logic]
  max-cognitive-complexity = 5
  ```

  ```python ok
  def load(text: str) -> str:
      if text:
          return _first(text)
      return ""


  def _first(text: str) -> str:
      for line in text.splitlines():
          if line:
              return line.strip()
      return ""
  ```

- **Branching helpers called from a test**: [conditional-in-test](SMP803.html) reports the branches of a test function,
  so inlining a helper with an `if`, `match` or conditional expression would move that branch into the test. A
  conditional that only forms a comprehension's item does not count, since conditional-in-test exempts it.

  ```python ok file=tests/test_header.py
  def test_kind() -> None:
      assert _expected("a") == 1


  def _expected(kind: str) -> int:
      if kind == "a":
          return 1
      return 2
  ```

- **A caller that the inlined helper might push over a limit of a Ruff rule**: Ruff's complexity and size rules count
  the inlined statements too, so a caller one of them might report stays as it is. Each limit counts only while its rule
  is selected in Ruff: `lint.mccabe.max-complexity` for C901 `complex-structure`, `lint.pylint.max-branches` for PLR0912
  `too-many-branches`, `lint.pylint.max-statements` for PLR0915 `too-many-statements`, and `lint.pylint.max-returns` for
  PLR0911 `too-many-return-statements`.

  While any of those four rules runs, a call in a conditional expression or in a later `and`/`or` operand is left alone,
  since inlining it would need a new `if`. A helper that is a lone `return` is reported anyway, as it inlines as an
  expression.

  ```toml config
  [tool.ruff.lint]
  select = ["C901"]

  [tool.ruff.lint.mccabe]
  max-complexity = 1
  ```

  ```python ok
  def load(text: str) -> str:
      return _first(text)


  def _first(text: str) -> str:
      if text:
          return text.strip()
      return ""
  ```

- **Coroutines**: an `async def` helper returns an awaitable, so its caller would await the result of the inlined code
  instead.

  ```python ok
  async def load(text: str) -> str:
      return await _parse_header(text)


  async def _parse_header(text: str) -> str:
      first = text.splitlines()[0]
      return first.strip()
  ```

- **Generators**: a helper with `yield` runs only when iterated, so inlining it would turn its caller into a generator.

  ```python ok
  def load(text: str) -> list[str]:
      return list(_lines(text))


  def _lines(text: str):
      for line in text.splitlines():
          yield line.strip()
  ```

- **Type guards**: a helper returning `TypeGuard[...]` or `TypeIs[...]`, from `typing` or `typing_extensions` under any
  alias, narrows the caller's type where it is called, and the bare expression inlined in its place would not, so the
  caller would fail type checking.

  ```python ok
  from typing import TypeGuard


  def load(value: object) -> str:
      if _is_text(value):
          return value
      return ""


  def _is_text(value: object) -> TypeGuard[str]:
      return isinstance(value, str)
  ```

### Fix

The rule has no fix: inlining needs the helper's parameters renamed to the caller's arguments, which the rule does not
attempt.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
