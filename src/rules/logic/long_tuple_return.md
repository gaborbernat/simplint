Flags a `return` statement with more than two tuple elements by default. When the elements are fields of a record, a
dataclass or named tuple gives each one a name.

## Why is this bad?

Callers unpack a tuple by position, so a reader must find the function to learn what each element means. Past two or
three elements that is easy to get wrong: swapping two `int`s type-checks and runs. A caller can read `span.start`
directly in place of `span[0]`. The
[`namedtuple` docs](https://docs.python.org/3/library/collections.html#collections.namedtuple) describe that as
self-documenting code.

The standard library uses named fields for results such as
[`os.stat()` and `time.localtime()`](https://docs.python.org/3/glossary.html#term-named-tuple) and
[`urllib.parse.urlparse()`](https://docs.python.org/3/library/urllib.parse.html#urllib.parse.urlparse). A named tuple
still unpacks and indexes like a tuple, so callers that unpack it keep working.

## Example

```python report
def span(text: str) -> tuple[int, int, str]:
    start = text.find("(")
    end = text.rfind(")")
    return start, end, text[start:end]
```

Use instead:

```python ok
from typing import NamedTuple


class Span(NamedTuple):
    start: int
    end: int
    text: str


def span(text: str) -> Span:
    start = text.find("(")
    end = text.rfind(")")
    return Span(start, end, text[start:end])
```

## Configuration

- [`logic.max-tuple-elements`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#logic) in
  `[tool.simplint]`: the most elements a returned tuple may hold, such as `max-tuple-elements = 3` under
  `[tool.simplint.logic]`

  ```toml config
  [tool.simplint.logic]
  max-tuple-elements = 3
  ```

  ```python report
  def corners() -> tuple[int, int, int, int]:
      return 0, 0, 1, 1
  ```

## What it does

### Active on

The rule checks tuple literals in each function's own `return` statements.

- **A tuple longer than the limit**, with or without parentheses.

  ```python report
  def bounds(values: list[int]) -> tuple[int, int, int]:
      return min(values), max(values), len(values)
  ```

- **A nested function**: its return is judged on its own.

  ```python report
  def outer() -> int:
      def inner() -> tuple[int, int, int]:
          return 1, 2, 3

      return inner()[0]
  ```

### Exceptions

- **Functions or methods decorated with `@override`.** The rule matches the final name `override`, including in
  [`@typing.override`](https://docs.python.org/3/library/typing.html#typing.override). A base class may require the
  method to return a particular shape.

  ```python ok
  from typing import override

  from parsing import Base


  class Parser(Base):
      @override
      def span(self, text: str) -> tuple[int, int, int]:
          return 0, len(text), 1
  ```

- **A method that redefines one a base class of the run defines**: the base sets the shape, so the override keeps it.

  ```python ok
  class Source:
      def field_value(self, name: str) -> tuple[object, str, bool]:
          raise NotImplementedError


  class EnvSource(Source):
      def field_value(self, name: str) -> tuple[object, str, bool]:
          return None, name, False
  ```

- **The methods the pickle and copy protocols call**: `__reduce__`, `__reduce_ex__`, `__getnewargs__` and
  `__getnewargs_ex__` must return a tuple of the shape those protocols read.

  ```python ok
  class Point:
      def __reduce__(self):
          return self.__class__, (self.x,), self.__dict__
  ```

- **`__getstate__` and `getstate`.** Pickle and incremental codecs can pass this state back to `__setstate__` or
  `setstate`. The rule exempts the function names without checking the enclosing class.

  ```python ok
  class Point:
      def __getstate__(self):
          return self.x, self.y, self.z
  ```

- **A key function**, named `key` or ending in `_key`: sorting and caching often use the tuple as a single key.

  ```python ok
  class Version:
      def _sort_key(self):
          return self.major, self.minor, self.micro
  ```

- **A function annotated to return `tuple[T, ...]`**: its tuple is a sequence of any length rather than a record.

  ```python ok
  def primes() -> tuple[int, ...]:
      return 2, 3, 5, 7
  ```

- **A function annotated with a quoted `"tuple[T, ...]"`**: the annotation is read as code, as a forward reference is,
  so it counts like the unquoted one.

  ```python ok
  def primes() -> "tuple[int, ...]":
      return 2, 3, 5, 7
  ```

- **A tuple with a starred element**: the number of values it stands for is not known.

  ```python ok
  def first(items: list[int]) -> tuple[int, ...]:
      return (*items, 1, 2)
  ```

- **A `yield`** produces a stream item. The rule only checks `return` statements.

  ```python ok
  def rows() -> Iterator[tuple[int, int, int]]:
      yield 1, 2, 3
  ```

### Fix

No fix is offered: a record needs a name for its fields, which only the author can choose.

## Ruff overlap

Ruff has no rule for the length of a returned tuple.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** issues [#3845](https://github.com/astral-sh/ruff/issues/3845) and
  [#8022](https://github.com/astral-sh/ruff/issues/8022) track the wider wemake-python-styleguide ruleset and its
  complexity checks.
- [flake8-too-many](https://github.com/queensferryme/flake8-too-many) counts return values as
  [TMN002](https://github.com/queensferryme/flake8-too-many) against `max-function-return-values`, which defaults to 3.
  This rule's limit defaults to 2.
- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports long tuples in both
  `return` and `yield` as
  [WPS227](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/complexity.html#wemake_python_styleguide.violations.complexity.TooLongOutputTupleViolation),
  with no fix.
