Flags dataclasses declared without `kw_only=True`, and the calls that pass their fields by position.

## Why is this bad?

Positional construction ties every caller to the field order, so reordering or inserting a field silently shifts values
into the wrong fields. Keyword-only fields name each value at the call site.

## Example

```python report
from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class Release:
    name: str
    version: str


Release("simplint", "1.0.0")
```

Use instead:

```python ok
from dataclasses import dataclass


@dataclass(frozen=True, slots=True, kw_only=True)
class Release:
    name: str
    version: str


Release(name="simplint", version="1.0.0")
```

## Configuration

- [`target-version`](https://docs.astral.sh/ruff/settings/#target-version) and
  [`per-file-target-version`](https://docs.astral.sh/ruff/settings/#per-file-target-version), falling back to
  [`project.requires-python`](https://packaging.python.org/en/latest/specifications/pyproject-toml/#requires-python);
  see [Python version](https://simplint.readthedocs.io/en/latest/configuration/project.html#python-version)

  ```toml config
  [tool.ruff]
  target-version = "py39"
  ```

  ```python ok
  from dataclasses import dataclass


  @dataclass
  class Release:
      name: str
  ```

## What it does

### Active on

- **[Dataclass](https://docs.python.org/3/library/dataclasses.html#dataclasses.dataclass) decorators**: a decorator
  spelled `@dataclass` or `@dataclasses.dataclass` without `kw_only=True`. Any value other than a literal `True`, such
  as `kw_only=False`, is reported too, since only a literal `True` shows that every field is keyword-only. The report
  points at each call in the run that passes the fields by position. Its fix sets `kw_only=True` and passes those
  arguments by keyword, but only when every such call is in the same module. The forms are:

  - `@dataclass(frozen=True, slots=True)`: no `kw_only` option, so it is reported.

    ```python report
    from dataclasses import dataclass


    @dataclass(frozen=True, slots=True)
    class Release:
        name: str
        version: str
    ```

  - `@dataclass(**DC_KW)`: a `**` splat of a module-level dict literal, or of a `dict(...)` call with keyword arguments,
    counts the options its keys set. A splat simplint cannot read leaves the class unreported, as for the other
    dataclass rules.

  - `@dataclass(frozen=True, slots=True, kw_only=False)`: a value other than a literal `True`.

    ```python report
    from dataclasses import dataclass


    @dataclass(frozen=True, slots=True, kw_only=False)
    class Release:
        name: str
        version: str
    ```

  - `@dataclass(frozen=True, slots=True, kw_only=True)`: accepted.

    ```python ok
    from dataclasses import dataclass


    @dataclass(frozen=True, slots=True, kw_only=True)
    class Release:
        name: str
        version: str
    ```

  **Unsafe fix:** code outside the checked files, such as the users of a published library, can still call the class by
  position. A library's public dataclasses are not reported at all, as the exceptions below describe.

- **Positional calls in other files**: each call in another file that passes the fields by position is a related
  location of the class's report, and the report gets no fix, since a rewrite does not reach another file. The message
  counts the calls that need converting by hand. A call gets no report of its own, so a suppression comment on the class
  silences the report with its related locations.

  ```python report file=app/models.py
  from dataclasses import dataclass


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str
      version: str
  ```

  ```python ok file=app/use.py
  from app.models import Release


  def build() -> Release:
      return Release("simplint", "1.0.0")
  ```

  The full output lists the call as a related location of the class report:

  ```text
  SMP703 Dataclass is missing kw_only; set `kw_only=True`; 1 positional call in another file needs keyword arguments by hand
   --> app/models.py:4:2
    |
  4 | @dataclass(frozen=True, slots=True)
    |  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  5 | class Release:
  6 |     name: str
    |
   ::: app/use.py:5:12
    |
  4 | def build() -> Release:
  5 |     return Release("simplint", "1.0.0")
    |            ---------------------------- positional arguments pass the fields by position

  Found 1 error.
  ```

### Exceptions

- **Public API of a library**: a dataclass that a public module offers, defined in or re-exported from it, where its
  `__all__` lists it or, without one, its name is public. Also a dataclass a public name's signature refers to, such as
  the base class of a listed class or a return annotation. Users of the library call it by position, so neither the
  class nor its positional calls in other files are reported. A class a module with an `__all__` leaves out is still
  reported, unless such a signature refers to it.

  ```toml config
  [build-system]
  requires = []
  ```

  ```python ok
  from dataclasses import dataclass

  __all__ = ["Release"]


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str
  ```

- **Target versions before 3.10**: the `kw_only` option needs Python 3.10 or later, so nothing is reported when the
  target version is older.

  ```toml config
  [tool.ruff]
  target-version = "py39"
  ```

  ```python ok
  from dataclasses import dataclass


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str
  ```

### Fix

The fix sets `kw_only=True` and rewrites the positional calls of the class in its own module, including `cls(...)` in
its classmethods. A `cls` parameter of an ordinary method is some other value, so its calls stay as they are. A call
that unpacks `**kwargs` passes those fields by keyword, so it keeps working.

A call that has positional arguments and a `**` splat is rewritten only when the splat is one simplint can read and it
does not pass one of the fields the rewrite names, since the keyword would repeat it. The class gets no fix when one of
its calls is not rewritten, so its other calls keep their positional arguments.

```python report
from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class Release:
    name: str

    @classmethod
    def named(cls, name: str) -> "Release":
        return cls(name)
```

```python fixed
from dataclasses import dataclass


@dataclass(frozen=True, slots=True, kw_only=True)
class Release:
    name: str

    @classmethod
    def named(cls, name: str) -> "Release":
        return cls(name=name)
```

The `cls` of an ordinary method is left alone:

```python report
from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class Release:
    name: str

    def build(self, cls: type, name: str) -> object:
        return cls(name)
```

```python fixed
from dataclasses import dataclass


@dataclass(frozen=True, slots=True, kw_only=True)
class Release:
    name: str

    def build(self, cls: type, name: str) -> object:
        return cls(name)
```

A positional call inside an annotation, as `Doc("...")` in `Annotated[str, Doc("...")]`, runs with the others, and a
positional walrus becomes the value of its keyword in parentheses:

```python report
from dataclasses import dataclass
from typing import Annotated


@dataclass
class Doc:
    text: str


def load(path: Annotated[str, Doc("where to read from")]) -> None:
    print(path)
```

```python fixed
from dataclasses import dataclass
from typing import Annotated


@dataclass(kw_only=True)
class Doc:
    text: str


def load(path: Annotated[str, Doc(text="where to read from")]) -> None:
    print(path)
```

```python report
from dataclasses import dataclass


@dataclass
class Release:
    name: str
    version: str


def build(raw: str) -> Release:
    return Release(name := raw.strip(), version=raw)
```

```python fixed
from dataclasses import dataclass


@dataclass(kw_only=True)
class Release:
    name: str
    version: str


def build(raw: str) -> Release:
    return Release(name=(name := raw.strip()), version=raw)
```

A call in a string annotation is rewritten like one in the code, as `Doc("where")` in `"Annotated[str, Doc('where')]"`:

```python report
from dataclasses import dataclass
from typing import Annotated


@dataclass
class Doc:
    text: str


def load(path: "Annotated[str, Doc('where')]") -> None:
    print(path)
```

```python fixed
from dataclasses import dataclass
from typing import Annotated


@dataclass(kw_only=True)
class Doc:
    text: str


def load(path: "Annotated[str, Doc(text='where')]") -> None:
    print(path)
```

The keywords follow the field order of the generated `__init__`:

- inherited fields come first

  ```python report
  from dataclasses import dataclass


  @dataclass(frozen=True, slots=True)
  class Base:
      name: str


  @dataclass(frozen=True, slots=True)
  class Release(Base):
      version: str


  Release("simplint", "1.0.0")
  ```

- `ClassVar` fields are left out, as are fields with `init=False` or `kw_only=True` and the fields after `KW_ONLY`

  ```python report
  from dataclasses import dataclass


  from dataclasses import KW_ONLY, dataclass, field
  from typing import ClassVar


  @dataclass(frozen=True, slots=True)
  class Release:
      count: ClassVar[int] = 0
      name: str
      cached: str = field(init=False, default="")
      _: KW_ONLY
      version: str


  Release("simplint", version="1.0.0")
  ```

- `InitVar` fields are included

  ```python report
  from dataclasses import dataclass


  from dataclasses import InitVar, dataclass


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str
      token: InitVar[str]


  Release("simplint", "secret")
  ```

No fix is offered while any use could still pass fields by position, or while a positional call is in another file:

- a call that unpacks `*args`, or passes more arguments than the class has positional fields;

  ```python report
  from dataclasses import dataclass


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str


  def build(*args: str) -> Release:
      return Release(*args)
  ```

- the class used as a value, as in
  [`functools.partial(Partition, topic)`](https://docs.python.org/3/library/functools.html#functools.partial), matched
  by position in a `case` pattern, or named in the metadata of an `Annotated[...]` type, where a caller may call it by
  position later;

  ```python report
  from dataclasses import dataclass
  from typing import Annotated


  @dataclass
  class Doc:
      text: str


  def load(path: Annotated[str, Doc]) -> None:
      print(path)
  ```

  ```python report
  from dataclasses import dataclass
  from functools import partial


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str
      version: str


  make = partial(Release, "simplint")
  ```

- the class read by [`getattr(module, "Name")`](https://docs.python.org/3/builtins/functions.html#getattr) from another
  file, which calls it by position under a name no import shows;

  ```python report file=app/models.py
  from dataclasses import dataclass


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str
      version: str
  ```

  ```python context file=app/main.py
  import app.models


  Release = getattr(app.models, "Release")("simplint", "1")
  ```

- a subclass, or a call through `type(self)(...)` or `self.__class__(...)`;

  ```python report
  from dataclasses import dataclass


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str

      def copy(self) -> "Release":
          return type(self)(self.name)
  ```

- a file that imports the module with `*`, or that imports the class and passes it on to other modules;

  ```python report file=app/models.py
  from dataclasses import dataclass


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str
      version: str
  ```

  ```python context file=app/use.py
  from app.models import *


  Release("simplint", "1.0.0")
  ```

- a positional call simplint cannot tie to the class, such as through a local name that shadows the import, or of an
  attribute with the class's name on something no import binds, such as `module.Partition("a", 1)` on a module a test
  loads from its file path;

  ```python report
  from dataclasses import dataclass


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str
      version: str


  def build(Release: type) -> object:
      return Release("simplint", "1.0.0")
  ```

- a class one of the implicitly concatenated parts of a string annotation spells, as `"Annotated[str, " "Doc('x')]"`:
  Ruff parses such a string without the positions a rewrite needs

  ```python report
  from dataclasses import dataclass
  from typing import Annotated


  @dataclass
  class Doc:
      text: str


  def load(path: "Annotated[str, Doc('where')]") -> None:
      print(path)
  ```

- a positional call with a suppression comment for SMP703 on the call, which the fix would rewrite regardless;

  ```python report
  from dataclasses import dataclass


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str
      version: str


  Release("simplint", "1.0.0")  # noqa: SMP703
  ```

- a class in one of these forms:

  - nested in a function or another class.

    ```python report
    from dataclasses import dataclass


    def build() -> object:
        @dataclass(frozen=True, slots=True)
        class Release:
            name: str

        return Release("simplint")
    ```

  - that defines `__init__` or `__new__`.

    ```python report
    from dataclasses import dataclass


    @dataclass(frozen=True, slots=True)
    class Release:
        name: str

        def __init__(self, name: str) -> None:
            self.name = name
    ```

  - that sets `init=False`.

    ```python report
    from dataclasses import dataclass


    @dataclass(frozen=True, slots=True, init=False)
    class Release:
        name: str
    ```

  - that has a base class that is not a dataclass of the same module.

    ```python report
    from dataclasses import dataclass


    class Base:
        name: str


    @dataclass(frozen=True, slots=True)
    class Release(Base):
        version: str
    ```

A call in an f-string `=` debug field keeps the printed text: the call's source text stays, and its value follows in a
plain field, so the output is the same once the call passes its fields by keyword.

```python report
from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class Release:
    name: str


def show() -> str:
    return f"{Release('simplint')=}"
```

```python fixed
from dataclasses import dataclass


@dataclass(frozen=True, slots=True, kw_only=True)
class Release:
    name: str


def show() -> str:
    return f"Release('simplint')={Release(name='simplint')!r}"
```

### Related rules

[dataclass-frozen](SMP701.html) asks for `frozen=True` and [dataclass-slots](SMP702.html) for `slots=True` on the same
decorator.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issue [#28777](https://github.com/astral-sh/ruff/issues/28777).
