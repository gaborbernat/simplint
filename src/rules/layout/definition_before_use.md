Flags functions and classes that do not sit directly below their first user.

## Why is this bad?

Keeping a helper near its first caller lets a reader follow a call without searching the file. Python resolves names in
a function body [when the function runs](https://docs.python.org/3/reference/executionmodel.html#resolution-of-names),
so helpers can often follow their callers. Names evaluated at import time, such as decorators and base classes, must
already exist.

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

  - a function a class body calls, such as `tokens = [state(1)]`; reading a method, as `__str__ = __repr__` or
    `property(_get)` do, calls nothing

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
  where it sits, so the definition reading it stays below it.

  ```python ok
  def main() -> object:
      return _load("{}")


  import json


  def _load(text: str, loads=json.loads) -> object:
      return loads(text)
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

- **At module level, a definition whose own statement reads another module-level name that is not an import**, for
  example in a decorator or default value, moves only where that name is still bound above it.

  ```python ok
  def main() -> int:
      return _load()


  LIMIT = 3


  def _load(limit: int = LIMIT) -> int:
      return limit
  ```

- **Everything a definition that runs at import time calls**: it runs at import time too. Naming a definition as a base
  class or a default value does not run its body, so the helpers a base class's methods call still move.

  ```python ok
  def _base() -> int:
      return 2


  def _scaled() -> int:
      return _base() * 2


  LIMIT = _scaled()
  ```

### Annotations

Annotations do not make a user when Python evaluates them lazily:

- when the target version is 3.14 or later ([PEP 649](https://peps.python.org/pep-0649/)), using the project's
  [Python version settings](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)
- when the module has [`from __future__ import annotations`](https://docs.python.org/3/library/__future__.html)

Otherwise an annotation in a module-level or class-level `def` signature or variable annotation counts as an import-time
use. Annotations inside a function body never make that function a user: a name a function only annotates with is not a
helper it calls.

A decorator can inspect annotations when the module loads. simplint therefore treats references in decorated function
signatures and decorated class fields as import-time uses, unless it recognizes the decorator. The exceptions include
property accessors, `classmethod`, `staticmethod`, `abstractmethod`, `contextmanager`, `asynccontextmanager`,
`dataclass`, `functools` wrappers such as `cache`, typing decorators such as `override`, `mock.patch` and its `dict`,
`multiple` and `object` variants, and pytest's `fixture`, `hookimpl`, `hookspec` and `mark` decorators.

### Type parameters and type statements

Python 3.12 introduced lazy evaluation for type parameter bounds and `type` statements
([PEP 695](https://peps.python.org/pep-0695/)). Python 3.13 added lazy type parameter defaults
([PEP 696](https://peps.python.org/pep-0696/)). Names used in these expressions can be defined later. They make their
statement a user like a function body does:

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

- A comment indented deeper than the definition below it, such as a trailing note at the end of the previous method,
  stays where it is, since it belongs to the code above it.

  ```python report
  # fmt: off
  def first() -> int:
      return 1
      # trailing note of first
  def _clean(line: str) -> str:
      return line.strip()


  def load(text: str) -> list[str]:
      return [_clean(line) for line in text.splitlines()]
  # fmt: on
  ```

  ```python fixed
  # fmt: off
  def first() -> int:
      return 1
      # trailing note of first
  def load(text: str) -> list[str]:
      return [_clean(line) for line in text.splitlines()]


  def _clean(line: str) -> str:
      return line.strip()
  # fmt: on
  ```

- A line of a string that starts with `#` is not a comment, so it stays with the string.

  ```python report
  # fmt: off
  MESSAGE = """
  # not a comment"""
  def _clean(line: str) -> str:
      return line.strip()


  def load(text: str) -> list[str]:
      return [_clean(line) for line in text.splitlines()]
  # fmt: on
  ```

  ```python fixed
  # fmt: off
  MESSAGE = """
  # not a comment"""
  def load(text: str) -> list[str]:
      return [_clean(line) for line in text.splitlines()]


  def _clean(line: str) -> str:
      return line.strip()
  # fmt: on
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

simplint marks the fix unsafe when it moves a class, a function with a call in a default value or decorator, or a
function with a decorator outside the recognized set. It accepts `property`, `cached_property`, `classmethod`,
`staticmethod`, `overload`, `setter`, `getter` and `deleter` by their final name. A definition that keeps its position
in the rewritten span does not affect the safety classification.

No fix is offered when the span holds a comment that is not attached to a definition, or when two statements share a
line.

## Other tools

- [ssort](https://github.com/bwhmather/ssort) puts definitions before their users.
- [pyorder](https://github.com/Winipedia/pyorder) orders module-level definitions with the same
  [callers-before-callees](https://github.com/Winipedia/pyorder#ordering-rules) rule, and its `--fix` option rewrites
  the file. Ties break by visibility and then alphabetically, which this rule does not check.
- [flake8-stepdown](https://github.com/didmar/flake8-stepdown) reports functions defined in the wrong order as
  [TDP001](https://github.com/didmar/flake8-stepdown#violation-codes), and its `stepdown fix` command rewrites them.
