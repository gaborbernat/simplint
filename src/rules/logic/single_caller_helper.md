Flags a private module-level function that only one other function calls, when inlining its statements into that caller
keeps the caller short.

## Why is this bad?

A helper with a single caller splits one piece of work over two functions. A reader has to jump to the helper to learn
what the caller does, and the helper's name adds little when only one place uses it. Inlining the helper's statements
keeps the work in one function, as long as the caller stays short enough to read at once.

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

- **Helpers passed as a value**: `map(_parse_header, texts)` does not call the helper where it stands.

  ```python ok
  def load(texts: list[str]) -> list[str]:
      return list(map(_parse_header, texts))


  def _parse_header(text: str) -> str:
      first = text.splitlines()[0]
      return first.strip()
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

- **Helpers with `max-helper-statements` or more statements**: a longer helper is worth its name.

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

### Fix

The rule has no fix: inlining needs the helper's parameters renamed to the caller's arguments, which the rule does not
attempt.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet.
