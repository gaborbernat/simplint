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

- **In every class body**: a method refers to another through its first parameter, such as `self.name` or `cls.name`,
  through `ClassName.name`, or through an instance the method builds from its first parameter, such as `obj = cls()` or
  `obj = super().__new__(cls)` followed by `obj.name`. A lambda in the class body, such as
  `property(lambda self: self.name)`, counts as a method.

  The first user is the one the class layout puts first, as [class-member-order](SMP203.html) orders the members: a
  constructor, an alternative constructor or a dunder method comes before the methods, so a helper that both a method
  and a constructor call follows the constructor.

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

  - a `def` in a block of module-level code, such as the body of an `if`, `try`, `with`, `for`, `while` or `match`
    statement, which the block may call while the module loads

    ```python ok
    def _render(value: int) -> str:
        return str(value)


    if __name__ == "__main__":

        def main() -> None:
            print(_render(1))

        main()
    ```

- **Names read from an import placed below the first definition**, such as a default value: the import is bound only
  where it sits, so the definition reading it and the helpers it calls stay where they are.

  ```python ok
  def main() -> object:
      return _load("{}")


  def _check(text: str) -> str:
      return text.strip()


  import json


  def _load(text: str, loads=json.loads) -> object:
      return loads(_check(text))
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

- **Constructors**: `__new__`, `__init__`, `__post_init__` and `__init_subclass__` are entry points, never helpers, so a
  method that calls one does not pull it below that method. [class-member-order](SMP203.html) keeps each constructor in
  its section, above the methods.

  ```python ok
  class Cache:
      def __init__(self) -> None:
          self.items = {}

      def reset(self) -> None:
          self.__init__()
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

### Type parameters and type statements

Python 3.12 evaluates the bound and default of a type parameter ([PEP 695](https://peps.python.org/pep-0695/),
[PEP 696](https://peps.python.org/pep-0696/)) and the value of a `type` statement lazily, so the names they read need
not exist yet. They make their statement a user like a function body does:

```toml config
[project]
requires-python = ">=3.12"
```

```python report
class _Base:
    pass


def make[T: _Base]() -> T:
    raise NotImplementedError
```

### Fix

The fix rewrites the span from the first to the last misplaced statement in the expected order:

- It comes with the first report of each body only, since that one fix moves every definition the body's reports name.
  When a comment suppresses that report, the first report still shown carries it. In a class body the fix leaves the
  sections alone; [class-member-order](SMP203.html) sorts them, with the same helper placement.

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

- A line of a string that starts with `#` is not a comment, so it stays with the string.

  ```python report
  MESSAGE = """
  # not a comment"""


  def _clean(line: str) -> str:
      return line.strip()


  def load(text: str) -> list[str]:
      return [_clean(line) for line in text.splitlines()]
  ```

  ```python fixed
  MESSAGE = """
  # not a comment"""


  def load(text: str) -> list[str]:
      return [_clean(line) for line in text.splitlines()]


  def _clean(line: str) -> str:
      return line.strip()
  ```

- Statements are separated by two blank lines at module level and one in a class body. Two statements that stay adjacent
  in the same order keep the blank lines they had.

  ```python report
  class Parser:
      def _clean(self, line: str) -> str:
          return line.strip()

      def unrelated(self) -> int:
          return 1

      def load(self, text: str) -> list[str]:
          return [self._clean(line) for line in text.splitlines()]
  ```

  ```python report
  def _clean(line: str) -> str:
      return line.strip()


  WIDTH = 1
  HEIGHT = 2
  DEPTH = 3


  def load(text: str) -> list[str]:
      return [_clean(line) for line in text.splitlines()]
  ```

  ```python fixed
  WIDTH = 1
  HEIGHT = 2
  DEPTH = 3


  def load(text: str) -> list[str]:
      return [_clean(line) for line in text.splitlines()]


  def _clean(line: str) -> str:
      return line.strip()
  ```

The fix is safe when every definition in the rewritten span is an undecorated function. **Unsafe fix:** a span holding a
class or a decorated function, even when that definition only shifts to make room, because
[decorators](https://docs.python.org/3/glossary.html#term-decorator),
[metaclasses](https://docs.python.org/3/reference/datamodel.html#metaclasses) and
[`__init_subclass__`](https://docs.python.org/3/reference/datamodel.html#object.__init_subclass__) hooks then run in a
different order.

No fix is offered when the span holds a comment that is not attached to a definition, or when two statements share a
line.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** not requested yet. Open issue
  [#3946](https://github.com/astral-sh/ruff/issues/3946) asks for ssort, which orders the other way, with definitions
  above their users.
- [pyorder](https://github.com/Winipedia/pyorder) orders module-level definitions with the same
  [callers-before-callees](https://github.com/Winipedia/pyorder#ordering-rules) rule, and its `--fix` option rewrites
  the file. Ties break by visibility and then alphabetically, which this rule does not check.
- [flake8-stepdown](https://github.com/didmar/flake8-stepdown) reports functions defined in the wrong order as
  [TDP001](https://github.com/didmar/flake8-stepdown#violation-codes), and its `stepdown fix` command rewrites them.
