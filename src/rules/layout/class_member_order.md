Flags class members defined outside their section of the class layout. A class body reads in a fixed order: nested
classes, class attributes, constructors, alternative constructors, other dunder methods, properties, then the other
methods. A member that sits above a member of an earlier section is reported, and the fix moves it into place.

## Why is this bad?

A class is read to learn how its instances come to be, what they hold and what they can do. The layout puts each kind of
member where a reader looks for it. A constructor placed after a dozen methods sends the reader past all of them, and a
property defined between two methods splits the list of the methods the class offers.

The same order in every class means a reader who knows one class knows where to look in the next. Other guides order
members in a similar way.
[wemake-python-styleguide](https://wemake-python-styleguide.readthedocs.io/en/1.7.1/pages/usage/violations/consistency.html#wemake_python_styleguide.violations.consistency.WrongMethodOrderViolation)
wants the important parts before the details, and
[Django's model style](https://docs.djangoproject.com/en/dev/internals/contributing/writing-code/coding-style/#model-style)
lists fields, `Meta` and dunder methods before custom methods.

## Example

```python report
class Parser:
    def load(self, text: str) -> list[str]:
        return [line.strip() for line in text.splitlines()]

    def __init__(self, sep: str = ",") -> None:
        self.sep = sep

    def __repr__(self) -> str:
        return f"Parser({self.sep!r})"
```

Use instead:

```python ok
class Parser:
    def __init__(self, sep: str = ",") -> None:
        self.sep = sep

    def __repr__(self) -> str:
        return f"Parser({self.sep!r})"

    def load(self, text: str) -> list[str]:
        return [line.strip() for line in text.splitlines()]
```

## What it does

The rule checks the members of each class body, in the order of these sections:

1. **Nested classes**, such as `class Meta:`.
1. **Class attributes and fields**: the statements that are not definitions, such as `LIMIT = 3` or `__slots__`.
1. **Constructors**: `__init_subclass__`, `__new__`, `__init__` and `__post_init__`, in the order Python runs them.
1. **Alternative constructors**: classmethods that return a call of the class, such as `return cls(...)`, that return
   the result of another alternative constructor through `cls`, such as `return cls.fromordinal(...)`, or whose return
   annotation is `Self`, [`typing.Self`](https://docs.python.org/3/library/typing.html#typing.Self) or the class's own
   name. A returned local name counts as the call it holds when that assignment, such as `result = cls(...)`, is the
   only one outside a branch that binds the name; a branch may rebind it, as `result = tz.fromutc(result)` does.
1. **Other dunder methods**, such as `__repr__` and `__eq__`.
1. **Properties**: a `property` or `cached_property` with its setter and deleter right after it.
1. **Methods**: instance, static and class methods alike, public or private.

Members of a section keep their source order, so adding a method never reorders the file. Sections are not sorted by
name or by visibility. The docstring of the class stays first.

```python ok
class Layout:
    """Docstring."""

    class Meta:
        pass

    LIMIT = 3
    __slots__ = ("a",)

    def __init_subclass__(cls) -> None:
        pass

    def __new__(cls) -> "Layout":
        return super().__new__(cls)

    def __init__(self) -> None:
        self.a = 1

    def __post_init__(self) -> None:
        pass

    @classmethod
    def from_json(cls, raw: str) -> "Layout":
        return cls()

    def __repr__(self) -> str:
        return "Layout"

    @property
    def size(self) -> int:
        return 1

    @size.setter
    def size(self, value: int) -> None:
        pass

    def run(self) -> None:
        pass
```

### Active on

- **Alternative constructors**: a classmethod that hands its work to another alternative constructor is one too, and so
  is a classmethod annotated to return the class, whatever its body does:

  ```python report
  from typing import Self


  class Date:
      def __init__(self, ordinal: int) -> None:
          self.ordinal = ordinal

      def describe(self) -> str:
          return str(self.ordinal)

      @classmethod
      def fromordinal(cls, ordinal: int):
          return cls(ordinal)

      @classmethod
      def today(cls):
          return cls.fromordinal(1)

      @classmethod
      def parse(cls, text: str) -> Self:
          return cls.registry[text]

      @classmethod
      def epoch(cls, shift: bool = False):
          result = cls.fromordinal(0)
          if shift:
              result = result.shifted()
          return result

      registry: dict[str, "Date"] = {}
  ```

  ```python fixed
  from typing import Self


  class Date:
      registry: dict[str, "Date"] = {}

      def __init__(self, ordinal: int) -> None:
          self.ordinal = ordinal

      @classmethod
      def today(cls):
          return cls.fromordinal(1)

      @classmethod
      def fromordinal(cls, ordinal: int):
          return cls(ordinal)

      @classmethod
      def parse(cls, text: str) -> Self:
          return cls.registry[text]

      @classmethod
      def epoch(cls, shift: bool = False):
          result = cls.fromordinal(0)
          if shift:
              result = result.shifted()
          return result

      def describe(self) -> str:
          return str(self.ordinal)
  ```

- **Members out of section order**: a member in a section below a later section's member is reported. The message names
  the section it belongs with and the first member of a later section above which it sits. A property and its setter are
  reported on their own lines, since each is a member of its own:

  ```python report
  class Box:
      def fill(self, value: int) -> None:
          self._size = value

      @property
      def size(self) -> int:
          return self._size

      @size.setter
      def size(self, value: int) -> None:
          self._size = value
  ```

  ```python fixed
  class Box:
      @property
      def size(self) -> int:
          return self._size

      @size.setter
      def size(self, value: int) -> None:
          self._size = value

      def fill(self, value: int) -> None:
          self._size = value
  ```

### Exceptions

Some members stay where they are, and some reports come without a fix:

- **Helpers.** A method that another method of the class calls is a helper, not an entry point. It follows its first
  caller in the layout, whatever its section, as [`definition-before-use`](SMP201.html) places it. This rule does not
  report it:

  ```python ok
  class Parser:
      def load(self, text: str) -> list[str]:
          return self._split(text)

      def _split(self, text: str) -> list[str]:
          return text.splitlines()
  ```

- **A classmethod that calls a plain classmethod** stays a method: one that only returns a call of a plain classmethod
  is not an alternative constructor.

  ```python ok
  class Date:
      @property
      def day(self) -> int:
          return 1

      @classmethod
      def label(cls, ordinal: int) -> str:
          return cls.describe_ordinal(ordinal)

      @classmethod
      def describe_ordinal(cls, ordinal: int) -> str:
          return str(ordinal)
  ```

- **A returned name bound outside a branch** keeps a classmethod a method when the name is first bound outside a branch
  to something else, or bound twice outside a branch.

  ```python ok
  class Date:
      @classmethod
      def fromordinal(cls, ordinal: int):
          return cls(ordinal)

      @property
      def day(self) -> int:
          return 1

      @classmethod
      def epoch(cls, shift: bool = False):
          result = "epoch"
          if shift:
              result = cls.fromordinal(0)
          return result

      @classmethod
      def today(cls):
          result = cls.fromordinal(1)
          result = cls.fromordinal(2)
          return result
  ```

- **Constructors that a method calls** stay in their section. `self.__init__()` in a method does not make the
  constructor a helper, so the constructor keeps its place above the methods.

  ```python ok
  class Parser:
      def __init__(self, sep: str = ",") -> None:
          self.sep = sep

      def reset(self) -> None:
          self.__init__()

      def load(self, text: str) -> list[str]:
          return text.split(self.sep)
  ```

- **Overloads and redefinitions** of one name, and a property with its setter and deleter, move as one unit.

  ```python report
  from typing import overload


  class Box:
      def fill(self, value: int) -> None:
          self._size = value

      @overload
      def __init__(self, size: int) -> None: ...

      @overload
      def __init__(self, size: str) -> None: ...

      def __init__(self, size: int | str) -> None:
          self._size = int(size)
  ```

  ```python fixed
  from typing import overload


  class Box:
      @overload
      def __init__(self, size: int) -> None: ...

      @overload
      def __init__(self, size: str) -> None: ...

      def __init__(self, size: int | str) -> None:
          self._size = int(size)

      def fill(self, value: int) -> None:
          self._size = value
  ```

- **A name the class body reads while it runs** pins the definition that binds it. Default values, decorators and
  annotations that Python evaluates when the `def` runs count as reads: a definition cannot move below a reader of its
  name, and a reader that needs the name bound cannot move above it.

  ```python report
  class Parser:
      def load(self, text: str) -> list[str]:
          return text.split(self.SEP)

      SEP = ","

      def __init__(self, sep: str = SEP) -> None:
          self.sep = sep
  ```

  The constructor needs `SEP` bound when it is defined, and `SEP` sits below `load`, so the constructor stays where it
  is and the report has no fix.

- **Statements that are not definitions.** Such a statement sits in section 2, so it moves up with the other class
  attributes. It stays in place when it conflicts with a definition: it binds a name that a definition reads while the
  class is created, or it reads a name that a definition binds. A definition does not move past such a statement. A
  `match` capture or an `except ... as` name binds its name too.

  ```python report
  class Shape:
      def area(self) -> int:
          return 1

      match 1:
          case area:
              pass

      def __init__(self) -> None:
          self.size = 1
  ```

  ```python fixed
  class Shape:
      def area(self) -> int:
          return 1

      match 1:
          case area:
              pass

      def __init__(self) -> None:
          self.size = 1
  ```

- **Django model statements Ruff reports**: in a file that imports Django,
  [DJ012](https://docs.astral.sh/ruff/rules/django-unordered-body-content-in-model/) reports a statement of a model that
  the Django Style Guide places earlier, so this rule leaves that statement to Ruff. DJ012 has no fix, so the one report
  that carries the fix for the class stays here, on a member DJ012 does not check when there is one. Below, `__init__`
  is reported with the fix and `Meta` is left to Ruff:

  ```python report
  from django.db import models


  class Article(models.Model):
      def helper(self) -> None:
          pass

      def __init__(self) -> None:
          super().__init__()

      class Meta:
          ordering = ["name"]
  ```

- **An enum's `_generate_next_value_`** stays where it is. `auto()` reads it from the class namespace rather than by
  name, and `Enum` raises [`TypeError`](https://docs.python.org/3/builtins/exceptions.html#TypeError) when it is defined
  after a member that called `auto()`:

  ```python report
  from enum import Enum, auto


  class Direction(Enum):
      @staticmethod
      def _generate_next_value_(name: str, start: int, count: int, last_values: list[str]) -> str:
          return name

      NORTH = auto()

      def describe(self) -> str:
          return self.value

      def __str__(self) -> str:
          return self.name
  ```

  `NORTH` cannot move above `_generate_next_value_`, so `__str__` cannot move above it either, and the report has no
  fix.

### Fix

The fix rewrites the span from the first to the last moved statement in the expected order. Only the first report of a
class body that the layout can resolve carries it, since that one fix reorders every member of the body. A member keeps
its decorators and the comment lines directly above it. A comment indented deeper than the member below it belongs to
the member above, so it stays in place, and when it lies inside the span the report has no fix.

```python report
# fmt: off
class A:
    def run(self):
        return 1
        # trailing note of run
    def __init__(self):
        self.x = 1
# fmt: on
```

```python fixed
# fmt: off
class A:
    def run(self):
        return 1
        # trailing note of run
    def __init__(self):
        self.x = 1
# fmt: on
```

The fix is safe when every moved member is undecorated or has only pure decorators: `property`, `cached_property`,
`classmethod`, `staticmethod`, `overload`, and the accessor decorators `setter`, `getter` and `deleter`. **Unsafe fix:**
a moved class, a moved method with any other decorator, such as `@cache` or `@functools.lru_cache()`, or a call in a
moved method's default values or in a moved statement, because
[decorators](https://docs.python.org/3/glossary.html#term-decorator), calls and class creation then run in a different
order. A call can also see which names the class body has bound so far, as
[`locals()`](https://docs.python.org/3/builtins/functions.html#locals) does:

```python report
import itertools

counter = itertools.count()


class Counted:
    def method(self) -> int:
        return 1

    first = next(counter)

    def __repr__(self) -> str:
        return "Counted"
```

Two members that stay adjacent in the same order keep the blank lines they had between them. The other members are
separated by one blank line.

```python report
class Parser:
    def load(self) -> int:
        return 1

    LIMIT = 3
    SEP = ","

    def __init__(self) -> None:
        self.sep = SEP
```

```python fixed
class Parser:
    LIMIT = 3
    SEP = ","

    def __init__(self) -> None:
        self.sep = SEP

    def load(self) -> int:
        return 1
```

No fix is offered when the member stays behind a statement it conflicts with, when the span holds a comment that is not
attached to a definition, or when two statements share a line.

### With definition-before-use

[`definition-before-use`](SMP201.html) places the helpers, and this rule places the sections. Both compute one expected
order for a class body, so the two never disagree: each helper follows the same caller in both, and the sections are the
same in both. When both rules are enabled, one `--fix` settles a class body: the fix of either leads to the order the
other one wants, and a second run finds nothing.

When this rule is enabled alone, its fix moves each helper together with its caller, since the layout places the helper
there, but it reports no helper. When [`definition-before-use`](SMP201.html) is enabled alone, it does not sort the
sections, and its fix moves only the helpers.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** variation of released rule
  [DJ012](https://docs.astral.sh/ruff/rules/django-unordered-body-content-in-model/), which orders Django model bodies
  and which this rule leaves to it, except the report that carries the fix DJ012 lacks; the general order is open issues
  [#2425](https://github.com/astral-sh/ruff/issues/2425) and [#3845](https://github.com/astral-sh/ruff/issues/3845)
  (WPS338).
- [pyorder](https://github.com/Winipedia/pyorder) has the same
  [constructor-first](https://github.com/Winipedia/pyorder#ordering-rules) rule, and sorts abstract, overriding, special
  and class methods first as well.
- [flake8-class-attributes-order](https://github.com/best-doctor/flake8-class-attributes-order) reports
  [CCE001](https://github.com/best-doctor/flake8-class-attributes-order) when members are out of its default kind order.
  That order also puts properties, static and class methods, and private methods in fixed positions, so it can disagree
  with this rule.
