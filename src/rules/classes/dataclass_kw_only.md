Flags dataclasses whose decorator does not set `kw_only=True`. The diagnostic points to positional constructor calls
that simplint can identify.

## Why is this bad?

With positional arguments, swapping two fields can silently put the caller's values in the wrong fields. Keyword
arguments keep each value attached to its field name. Keyword-only fields can also put a required field after a field
with a default, including defaults inherited from a base class.
[Dataclass parameter ordering](https://docs.python.org/3/library/dataclasses.html#re-ordering-of-keyword-only-parameters-in-init)

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

- **Dataclass decorators** include aliases that resolve to
  [`dataclasses.dataclass`](https://docs.python.org/3/library/dataclasses.html#dataclasses.dataclass). Simplint accepts
  the decorator option only when its value is the literal `True`. This option sets the default for the class's own
  fields. Individual fields can override it.

  - `@dataclass(frozen=True, slots=True)`: no `kw_only` option, so it is reported.

    ```python report
    from dataclasses import dataclass


    @dataclass(frozen=True, slots=True)
    class Release:
        name: str
        version: str
    ```

  - `@dataclass(**DC_KW)` can supply options from a dict literal or a `dict(...)` call with named arguments. Simplint
    skips a class if it cannot read the keys of a `**` argument.

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

  The fix is unsafe. Callers outside the checked files may still pass fields by position.

- **Positional calls in other files** appear as related locations on the class diagnostic. Any such call prevents the
  fix because simplint can only edit one file at a time. A `# noqa: SMP703` on the class suppresses the diagnostic and
  its related locations.

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

- **`@dataclass(init=False)`** skips the generated constructor.

  ```python ok
  from dataclasses import dataclass


  @dataclass(init=False)
  class Event:
      code: int

      def __init__(self, code: int) -> None:
          self.code = code
  ```

- **Public library dataclasses** may have callers outside the project. Simplint skips a public class exported by a
  public module, directly or through a re-export. An explicit `__all__` determines which names the module exports. A
  class mentioned in an exported signature, for example as a base or return type, can also be part of the public API.

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

The unsafe fix sets `kw_only=True` and adds keywords to positional constructor calls in the class's module. This
includes calls through the first parameter of a classmethod, usually `cls`. A call that passes only keywords or
`**kwargs` needs no edit.

For a call with both positional arguments and `**kwargs`, simplint must be able to read the keys and rule out a
duplicate keyword. If it cannot rewrite every positional call, it leaves the class and all calls alone. A comment inside
the decorator also prevents the fix, as does a `**` argument that already supplies `kw_only=False` or another value
besides `True`.

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

An ordinary method can take a separate `cls` argument. Simplint leaves calls through that argument alone.

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

Simplint also rewrites constructor calls in annotations. In the example, `Doc("where")` becomes `Doc(text="where")`. An
assignment expression used as a keyword value needs parentheses.

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

Simplint can also rewrite a call inside a single string annotation.

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

Simplint uses the positional parameter order of the generated `__init__`.

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

- `ClassVar`, fields with `init=False` and keyword-only fields are excluded. `KW_ONLY` makes subsequent fields
  keyword-only unless they explicitly set `kw_only=False`.

  ```python report
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
  from dataclasses import InitVar, dataclass


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str
      token: InitVar[str]


  Release("simplint", "secret")
  ```

Simplint withholds the fix if any of the following applies.

- A call unpacks `*args` or passes more arguments than there are positional fields.

  ```python report
  from dataclasses import dataclass


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str


  def build(*args: str) -> Release:
      return Release(*args)
  ```

- Code passes the class as a value, for example to
  [`functools.partial`](https://docs.python.org/3/library/functools.html#functools.partial) or in `Annotated` metadata,
  or matches instances by position in a `case` pattern.

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

- Another file reads the class with
  [`getattr(module, "Name")`](https://docs.python.org/3/builtins/functions.html#getattr), which can hide calls from
  simplint.

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

- The class has a subclass or contains a call such as `type(self)(...)` or `self.__class__(...)`.

  ```python report
  from dataclasses import dataclass


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str

      def copy(self) -> "Release":
          return type(self)(self.name)
  ```

- Another file uses a star import from the module or re-exports the class.

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

- Simplint cannot resolve a possible positional call, for example because a local parameter shadows the class name or a
  test calls `module.Partition(...)` on a dynamically loaded module.

  ```python report
  from dataclasses import dataclass


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str
      version: str


  def build(Release: type) -> object:
      return Release("simplint", "1.0.0")
  ```

- An annotation containing the class name uses implicitly concatenated string literals. Simplint cannot map the call's
  arguments back to source positions.

  ```python report
  from dataclasses import dataclass
  from typing import Annotated


  @dataclass
  class Doc:
      text: str


  def load(path: "Annotated[str, Doc('where')]") -> None:
      print(path)
  ```

- A positional call in another file prevents the fix even if `# noqa: SMP703` hides that call from the diagnostic. A
  suppression on a call in the class's own module leaves the fix available. To suppress the diagnostic there, put the
  comment on the decorator.

  ```python report file=app/models.py
  from dataclasses import dataclass


  @dataclass(frozen=True, slots=True)
  class Release:
      name: str
      version: str
  ```

  ```python context file=app/use.py
  from app.models import Release

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

  - Its constructor fields cannot be resolved. Simplint can follow a previously defined dataclass in the same module and
    allows bases named `object`, `Generic`, `ABC` or `Protocol`. An unknown base, a computed `init` or `kw_only` field
    option, or a `**` argument to `field()` can prevent the fix.

    ```python report
    from dataclasses import dataclass


    class Base:
        name: str


    @dataclass(frozen=True, slots=True)
    class Release(Base):
        version: str
    ```

When a call appears in an f-string debug field (`=`), the fix preserves the original text shown before the value.

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

- Ruff's open [#28777](https://github.com/astral-sh/ruff/issues/28777) proposes checking the number of positional
  parameters in generated dataclass constructors.
