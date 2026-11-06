Flags dataclasses declared without `slots=True`.

## Why is this bad?

[Slots](https://docs.python.org/3/reference/datamodel.html#slots) reject writes to misspelled attributes, as an instance
has no `__dict__` to take a new name, and make each instance smaller.

## Example

```python report
from dataclasses import dataclass


@dataclass
class Release:
    version: str
```

Use instead:

```python ok
from dataclasses import dataclass


@dataclass(slots=True)
class Release:
    version: str
```

## What it does

### Active on

- **[Dataclass](https://docs.python.org/3/library/dataclasses.html#dataclasses.dataclass) decorators**: a decorator
  spelled `@dataclass` or `@dataclasses.dataclass` without `slots=True`. Any value other than a literal `True`, such as
  `slots=False`, is reported too, since only a literal `True` shows that the option is on.

  - `@dataclass`: `slots` is not set.

    ```python report
    from dataclasses import dataclass


    @dataclass
    class Release:
        version: str
    ```

  - `@dataclasses.dataclass(frozen=True)`: the dotted spelling, with `slots` missing.

    ```python report
    import dataclasses


    @dataclasses.dataclass(frozen=True)
    class Release:
        version: str
    ```

  - `@dataclass(slots=False)`: a value other than a literal `True`.

    ```python report
    from dataclasses import dataclass


    @dataclass(slots=False)
    class Release:
        version: str
    ```

  - `@dataclass(frozen=True, slots=True)`: `slots` is a literal `True`, so it is accepted.

    ```python ok
    from dataclasses import dataclass


    @dataclass(frozen=True, slots=True)
    class Release:
        version: str
    ```

### Exceptions

- **Target versions before 3.10**: nothing is reported, as `slots=` needs Python 3.10 or later.

  ```toml config
  [tool.ruff]
  target-version = "py39"
  ```

  ```python ok
  from dataclasses import dataclass


  @dataclass
  class Release:
      version: str
  ```

### Fix

The fix sets `slots=True` on the decorator. No fix is offered for:

- a class without fields, such as a test fixture standing for a dataclass with the default options

  ```python report
  from dataclasses import dataclass


  @dataclass
  class Empty:
      pass
  ```

- a class the module creates inside `with pytest.raises(...)` or `with self.assertRaises(...)`: the test expects code
  using the instance to fail, such as `{Key("a"): True}` raising because the class is unhashable, and slotting can
  change that

  ```python report
  from dataclasses import dataclass


  import pytest


  def test_key() -> None:
      with pytest.raises(TypeError):

          @dataclass
          class Key:
              name: str
  ```

Otherwise the fix leaves out `slots` when checked code shows that a slotted instance breaks the class.

```mermaid
flowchart TD
    fields{"Has fields?"}
    fields -->|no| none["No fix"]
    fields -->|yes| raises{"Created inside pytest.raises<br/>or assertRaises?"}
    raises -->|yes| none
    raises -->|no| breaks{"Does checked code<br/>break a slotted instance?"}
    breaks -->|yes| none
    breaks -->|no| add["Add slots=True"]
    classDef yellow fill:#f2d27a,stroke:#6b5410,color:#062d59
    classDef orange fill:#f0b48c,stroke:#8a3d12,color:#3d1606
    classDef green fill:#93cfa9,stroke:#1d5e3a,color:#0b2e1a
    class fields,raises,breaks yellow
    class none orange
    class add green
```

A fix that adds `slots`:

```python report
from dataclasses import dataclass


@dataclass
class Release:
    version: str
```

```python fixed
from dataclasses import dataclass


@dataclass(slots=True)
class Release:
    version: str
```

### When `slots=True` is left out

- the class declares `__slots__`

  ```python report
  from dataclasses import dataclass


  @dataclass(frozen=True)
  class Release:
      __slots__ = ("version",)
      version: str
  ```

- the class uses [`cached_property`](https://docs.python.org/3/library/functools.html#functools.cached_property), or has
  a subclass that does

  ```python report
  from dataclasses import dataclass


  from functools import cached_property


  @dataclass(frozen=True)
  class Release:
      version: str

      @cached_property
      def label(self) -> str:
          return self.version.upper()
  ```

- the class has a subclass with several bases, such as `class Context(FileContext, threading.local)`, whose layouts
  conflict with a slotted one

  ```python report
  from dataclasses import dataclass


  import threading


  @dataclass(frozen=True)
  class FileContext:
      path: str


  class Context(FileContext, threading.local):
      pass
  ```

- code sets an attribute that is not a field on `self` or on an instance known to be of the class, a `ClassVar` or
  `InitVar` attribute included, as a slotted instance has no `__dict__` to shadow it in

  ```python report
  from dataclasses import dataclass


  @dataclass(frozen=True)
  class Release:
      version: str

      def mark(self) -> None:
          self.marked = True
  ```

- code reaches the instance `__dict__`, as `self.__dict__.update(...)` or `vars(config)` do

  ```python report
  from dataclasses import dataclass


  @dataclass(frozen=True)
  class Release:
      version: str

      def load(self, values: dict[str, object]) -> None:
          self.__dict__.update(values)
  ```

- the class stays mutable and any checked file pickles with a protocol that may be 0 or 1, such as
  `pickle.dumps(obj, protocol)` in a loop over every protocol: those protocols refuse a slotted class without
  `__getstate__`, which a dataclass gets only when frozen. Here a method writes `self`, so `frozen` is left out too, and
  no fix is offered.

  ```python report
  import pickle
  from dataclasses import dataclass


  @dataclass
  class Release:
      version: str
      count: int = 0

      def bump(self) -> None:
          self.count += 1


  def dump(release: Release) -> None:
      for protocol in range(2):
          pickle.dumps(release, protocol)
  ```

- the class is frozen and has a subclass whose methods store attributes on `self`: the `__getstate__` a frozen slotted
  dataclass gets pickles only the fields and drops them

  ```python report file=app/models.py
  from dataclasses import dataclass


  @dataclass(frozen=True)
  class Release:
      version: str
  ```

  ```python context file=app/cache.py
  from app.models import Release


  class CachedRelease(Release):
      def store(self) -> None:
          self.value = 1
  ```

### Instances of the class

An object is **known to be of the class** when it is:

- `self` in a method of the class
- a name bound to a call of the class
- a name annotated with the class, `Optional[...]` and `... | None` included
- an attribute of `self` the class body annotates with it, such as `self.config` after `config: Config`

An object known to be a mock never counts, nor does anything reached from one. Mocks are names bound to `Mock()`,
`MagicMock()`, `create_autospec(...)`, `mocker.Mock()` or `mocker.patch(...)`, and `with patch(...) as name` targets.

Any other object is unknown, so a write through it counts only when it names a field or a `ClassVar` attribute. An
unannotated parameter such as `result` below is unknown: its write to `log` is not tied to `Result`, so `slots=True` is
still offered, and that write fails at runtime once the fix is applied.

```python report file=app/models.py
from dataclasses import dataclass


@dataclass
class Result:
    status: str
```

```python fixed file=app/models.py
from dataclasses import dataclass


@dataclass(slots=True)
class Result:
    status: str
```

```python context file=app/record.py
from app.models import Result


def record(result) -> None:
    result.log = "x"


record(Result("ok"))
```

### Unsafe fix

The fix is safe only when nothing else in the checked files might notice the change. It is unsafe when:

- the decorator already sets `frozen` or `slots` to something other than `True`;

  ```python report
  from dataclasses import dataclass


  @dataclass(frozen=False)
  class Release:
      version: str
  ```

  ```python fixed
  from dataclasses import dataclass


  @dataclass(frozen=False, slots=True)
  class Release:
      version: str
  ```

- the class has base classes;

  ```python report
  from dataclasses import dataclass


  class Base:
      pass


  @dataclass
  class Release(Base):
      version: str
  ```

  ```python fixed
  from dataclasses import dataclass


  class Base:
      pass


  @dataclass(slots=True)
  class Release(Base):
      version: str
  ```

- another class with the same name, defined in another module, gets attributes written on its instances or has a
  subclass that a slotted base breaks, as an import simplint cannot follow, such as through a re-export, may name this
  class;

  ```python report file=app/models.py
  from dataclasses import dataclass


  @dataclass
  class Release:
      version: str
  ```

  ```python fixed file=app/models.py
  from dataclasses import dataclass


  @dataclass(slots=True)
  class Release:
      version: str
  ```

  ```python context file=app/other.py
  class Release:
      pass


  release = Release()
  release.label = "x"
  ```

- the class body assigns or deletes an attribute on anything other than `self`, or mentions `super`, `__dict__`,
  `__weakref__`, `__setattr__` or `__delattr__`.

  ```python report
  from dataclasses import dataclass


  settings = object()


  @dataclass
  class Release:
      version: str
      settings.flag = True
  ```

  ```python fixed
  from dataclasses import dataclass


  settings = object()


  @dataclass(slots=True)
  class Release:
      version: str
      settings.flag = True
  ```

Code outside the checked files, and code that adds an attribute that is not a field to an object simplint cannot tie to
the class, is not detected, and breaks once the fix is applied.

### Related rules

[dataclass-frozen](SMP701.html) asks for `frozen=True` and [dataclass-kw-only](SMP703.html) asks for `kw_only=True` on
the same decorator.

## Other tools

- **[Ruff](https://github.com/astral-sh/ruff):** open issue [#8981](https://github.com/astral-sh/ruff/issues/8981).
