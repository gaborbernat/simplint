Flags functions and classes that do not sit directly below their first user. With each helper following the code that
calls it, the file reads top-down, from the entry point into the details.

## Why is this bad?

This order lets the reader start with the high-level overview, the entry point or caller, and then drill down into the
details, the helpers, as they read ahead, top to bottom.

Code reads top-down when each function is followed by the helpers it calls. A helper above its caller, or far from it,
makes the reader jump around the file.

The habit of putting helpers first comes from compiled languages such as C, where a function has to be
[declared before it is used](https://en.cppreference.com/c/language/function_declaration). Python has no such rule: a
name inside a function body is looked up
[when the function runs](https://docs.python.org/3/reference/executionmodel.html#resolution-of-names), not when it is
defined, so a helper can sit anywhere in the module.

That leaves the order free to serve the reader. A file that starts with its entry point and then explains each step lets
a reviewer follow the change the way it runs.

The order is depth first, like reading a book: each helper follows its first caller, and a helper's own helpers come
before the caller's next one. For this call tree the file reads `load`, `_parse`, `_split`, `_validate`:

- `load` calls `_parse` first, then `_validate`
  - `_parse` calls `_split`

## Example

```python report
def _parse(line: str) -> tuple[str, str]:
    key, _, value = line.partition("=")
    return key, value


def load(text: str) -> dict[str, str]:
    return dict(_parse(line) for line in text.splitlines())
```

Use instead:

```python ok
def load(text: str) -> dict[str, str]:
    return dict(_parse(line) for line in text.splitlines())


def _parse(line: str) -> tuple[str, str]:
    key, _, value = line.partition("=")
    return key, value
```

## What it does

### Active on

A helper that runs only when another definition is called should sit directly below its **first user**, the earliest
statement that refers to it. When one user has several helpers, they follow it in the order the user first refers to
them. The rule checks the order of functions and classes in two places:

- **At module level**: a user refers to a helper by name.

  ```python report
  def _clean(line: str) -> str:
      return line.strip()


  def unrelated() -> int:
      return 1


  def load(text: str) -> list[str]:
      return [_clean(line) for line in text.splitlines()]
  ```

- **In every class body**: a method refers to another through `self.name`, `cls.name` or `ClassName.name`.

  ```python report
  class Parser:
      def _clean(self, line: str) -> str:
          return line.strip()

      def unrelated(self) -> int:
          return 1

      def load(self, text: str) -> list[str]:
          return [self._clean(line) for line in text.splitlines()]
  ```

### Exceptions

Definitions that must exist while the module or class body runs stay where they are, since moving them below their user
would leave the name unbound when it is read:

- **Anything referenced at import time**: module-level code, decorators, default values and base classes.

  ```python ok
  def _limit() -> int:
      return 3


  LIMIT = _limit()


  def load() -> int:
      return LIMIT
  ```

- **Anything read by a function that runs at import time**: the name must already exist when that call runs, so it
  cannot move below it:

  - a lambda in module-level code or a class body; a lambda bound straight to a name counts as a definition instead:

    - called where it is written, such as `(lambda: options)()`:

      ```python ok
      def load() -> int:
          return 1


      RESULT = (lambda: load())()
      ```

    - handed to a call, such as `build(skip=lambda case: is_slow(case))`:

      ```python ok
      def is_slow(case: str) -> bool:
          return case == "slow"


      build(skip=lambda case: is_slow(case))
      ```

  - a function a class body calls, such as `tokens = [state(1)]`

    ```python ok
    def state(value: int) -> int:
        return value


    class Machine:
        tokens = [state(1)]
    ```

- **Names bound more than once**, such as a [property](https://docs.python.org/3/builtins/functions.html#property) and
  its setter: each later binding builds on or replaces the earlier one, so their order matters.

  ```python ok
  class Box:
      @property
      def size(self) -> int:
          return self._size

      @size.setter
      def size(self, value: int) -> None:
          self._size = value
  ```

- **At module level, definitions whose own statement evaluates another module-level name that is not an import**, for
  example in a decorator or default value: that name has to be bound first.

  ```python ok
  def load(limit: int = DEFAULT_LIMIT) -> int:
      return limit


  DEFAULT_LIMIT = 3
  ```

- **Everything these call**: it runs at import time too.

  ```python ok
  def _base() -> int:
      return 2


  def _scaled() -> int:
      return _base() * 2


  LIMIT = _scaled()
  ```

- **Mutually recursive helpers**: each is the other's first user, so no order puts both directly below their user.

  ```python ok
  def _even(n: int) -> bool:
      return True if n == 0 else _odd(n - 1)


  def _odd(n: int) -> bool:
      return False if n == 0 else _even(n - 1)
  ```

### Annotations

Annotations do not make a user when Python evaluates them lazily:

- when the target version is 3.14 or later ([PEP 649](https://peps.python.org/pep-0649/)), read from Ruff's
  [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) or
  [`requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python)
- when the module has [`from __future__ import annotations`](https://docs.python.org/3/library/__future__.html)

Otherwise an annotation in a module-level or class-level `def` signature or variable annotation counts as an import-time
use. Annotations inside a function body never make that function a user: a name a function only annotates with is not a
helper it calls.

The signature of a decorated function and the class-level annotations of a decorated class count as an import-time use
whatever the target version, because many decorators read them while the module loads:

- a [FastAPI](https://fastapi.tiangolo.com/) route builds its response model from the return annotation
- pydantic's `validate_call`, typer, click, [`dataclass`](https://docs.python.org/3/library/dataclasses.html), attrs and
  [`functools.singledispatch`](https://docs.python.org/3/library/functools.html#functools.singledispatch) read them too

Moving such a name below its user would leave the decorator with an unresolved forward reference.

### Fix

The fix rewrites the span from the first to the last misplaced statement in the expected order:

- It comes with the first report of each body only, since that one fix moves every definition the body's reports name.

  ```python report
  def _clean(line: str) -> str:
      return line.strip()


  def unrelated() -> int:
      return 1


  def load(text: str) -> list[str]:
      return [_clean(line) for line in text.splitlines()]
  ```

  ```python fixed
  def unrelated() -> int:
      return 1


  def load(text: str) -> list[str]:
      return [_clean(line) for line in text.splitlines()]


  def _clean(line: str) -> str:
      return line.strip()
  ```

- Decorators and the comment lines directly above a definition move with it.

  ```python report
  # Strips the line.
  def _clean(line: str) -> str:
      return line.strip()


  def unrelated() -> int:
      return 1


  def load(text: str) -> list[str]:
      return [_clean(line) for line in text.splitlines()]
  ```

  ```python fixed
  def unrelated() -> int:
      return 1


  def load(text: str) -> list[str]:
      return [_clean(line) for line in text.splitlines()]


  # Strips the line.
  def _clean(line: str) -> str:
      return line.strip()
  ```

- Statements are separated by two blank lines at module level and one in a class body.

  ```python report
  class Parser:
      def _clean(self, line: str) -> str:
          return line.strip()

      def unrelated(self) -> int:
          return 1

      def load(self, text: str) -> list[str]:
          return [self._clean(line) for line in text.splitlines()]
  ```

The fix is safe when every definition in the rewritten span is an undecorated function. **Unsafe fix:** a span holding a
class or a decorated function, even when that definition only shifts to make room, because
[decorators](https://docs.python.org/3/glossary.html#term-decorator),
[metaclasses](https://docs.python.org/3/reference/datamodel.html#metaclasses) and
[`__init_subclass__`](https://docs.python.org/3/reference/datamodel.html#object.__init_subclass__) hooks then run in a
different order.

No fix is offered when the span holds a comment that is not attached to a definition, or when two statements share a
line.
