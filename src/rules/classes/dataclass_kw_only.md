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
  as `kw_only=False`, is reported too, since only a literal `True` shows that every field is keyword-only. Its fix sets
  `kw_only=True` and passes every positional argument of the calls in the same module by keyword. The forms are:

  - `@dataclass(frozen=True, slots=True)`: no `kw_only` option, so it is reported.

    ```python report
    from dataclasses import dataclass


    @dataclass(frozen=True, slots=True)
    class Release:
        name: str
        version: str
    ```

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

  **Unsafe fix:** two kinds of caller can still call the class by position:

  - a caller whose fix simplint withdraws because it gives Ruff work it cannot fix
  - code outside the checked files, such as the users of a published library

- **Positional calls in other files**: in the other files simplint checks, each call that passes such a dataclass's
  fields by position gets its own report, so the fix can rewrite it to pass the arguments by keyword. That fix is safe,
  as the call does the same before and after.

  ```python report file=app/models.py
  from dataclasses import dataclass


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str
      version: str
  ```

  ```python fixed file=app/models.py
  from dataclasses import dataclass


  @dataclass(frozen=True, slots=True, kw_only=True)
  class Release:
      name: str
      version: str
  ```

  ```python report file=app/use.py
  from app.models import Release


  def build() -> Release:
      return Release("simplint", "1.0.0")
  ```

  ```python fixed file=app/use.py
  from app.models import Release


  def build() -> Release:
      return Release(name="simplint", version="1.0.0")
  ```

### Exceptions

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

The two fixes are applied in the same `--fix` run. The decorator fix also rewrites calls through `cls(...)` in the
class's own methods. A call that unpacks `**kwargs` passes those fields by keyword, so it keeps working.

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

Since the decorator fix counts on the call reports, no fix is offered while any use could still pass fields by position:

- a call that unpacks `*args`, or passes more arguments than the class has positional fields;

  ```python report
  from dataclasses import dataclass


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str


  def build(*args: str) -> Release:
      return Release(*args)
  ```

- the class used as a value, as in `functools.partial(Partition, topic)`, or matched by position in a `case` pattern;

  ```python report
  from dataclasses import dataclass
  from functools import partial


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str
      version: str


  make = partial(Release, "simplint")
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

- a positional call in a file whose own report cannot rewrite it: one that ignores `dataclass-kw-only`, does not have it
  in `fixable`, targets a Python version before 3.10, or has a suppression comment on the call;

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
