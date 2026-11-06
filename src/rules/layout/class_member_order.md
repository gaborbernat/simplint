Flags class members defined outside their section of the class layout. A class body reads in a fixed order: nested
classes, class attributes, constructors, alternative constructors, other dunder methods, properties, then the other
methods. A member that sits above a member of an earlier section is reported, and the fix moves it into place.

## Why is this bad?

A class is read to learn how its instances come to be, what they hold and what they can do. The layout puts each kind of
member where a reader looks for it: the constructors near the top, where the instance is made, the protocol methods
together, the properties next, and the methods last. A constructor placed after a dozen methods sends the reader past
all of them, and a property defined between two methods splits the list of the methods the class offers.

The same order in every class also means a reader who knows one class knows where to look in the next. It follows the
life of an object: it is created, it gets its protocol, and then it is used.

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
   annotation is `Self`, `typing.Self` or the class's own name. A returned local name counts as the call it holds when
   that assignment, such as `result = cls(...)`, is the only one outside a branch that binds the name; a branch may
   rebind it, as `result = tz.fromutc(result)` does.
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

A classmethod that hands its work to another alternative constructor is one too, and so is a classmethod annotated to
return the class, whatever its body does. A classmethod that only returns a call of a plain classmethod stays a method,
and so does one whose name is first bound outside a branch to something else, or bound twice outside a branch:

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

A member in a section below a later section's member is reported. The message names the section it belongs with and the
first member of a later section above which it sits. A property and its setter are reported on their own lines, since
each is a member of its own:

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

- **Constructors that a method calls** stay in their section. `self.__init__()` in a method does not make the
  constructor a helper, so the constructor keeps its place above the methods.

- **Overloads and redefinitions** of one name, and a property with its setter and deleter, move as one unit.

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
  class is created, or it reads a name that a definition binds. A definition does not move past such a statement.

- **An enum's `_generate_next_value_`** stays where it is. `auto()` reads it from the class namespace rather than by
  name, and `Enum` raises `TypeError` when it is defined after a member that called `auto()`:

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
its decorators and the comment lines directly above it.

The fix is safe when every moved member is undecorated or has only pure decorators: `property`, `cached_property`,
`classmethod`, `staticmethod`, `overload`, and the accessor decorators `setter`, `getter` and `deleter`. **Unsafe fix:**
a moved class, a moved method with any other decorator, such as `@cache` or `@functools.lru_cache()`, or a call in a
moved method's default values or in a moved statement, because
[decorators](https://docs.python.org/3/glossary.html#term-decorator), calls and class creation then run in a different
order. A call can also see which names the class body has bound so far, as `locals()` does:

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

- [pyorder](https://github.com/Winipedia/pyorder) has the same
  [constructor-first](https://github.com/Winipedia/pyorder#ordering-rules) rule, and sorts abstract, overriding, special
  and class methods first as well.
- [flake8-class-attributes-order](https://github.com/best-doctor/flake8-class-attributes-order) reports
  [CCE001](https://github.com/best-doctor/flake8-class-attributes-order) when members are out of its default kind order.
  That order also puts properties, static and class methods, and private methods in fixed positions, so it can disagree
  with this rule.
