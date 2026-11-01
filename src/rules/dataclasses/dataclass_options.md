## What it does

Checks for [dataclasses](https://docs.python.org/3/library/dataclasses.html#dataclasses.dataclass) declared without
`frozen=True` and `slots=True`. The decorator must be spelled `@dataclass` or `@dataclasses.dataclass`. Any value other
than a literal `True`, such as `frozen=False`, is reported too.

## Why is this bad?

Any code holding a reference to a mutable dataclass can change it, so a reader cannot trust a value they saw at
construction time. A [frozen](https://docs.python.org/3/library/dataclasses.html#frozen-instances) instance raises on
attribute assignment, and with the default `eq=True` it gets a `__hash__`, so it can go in a set or serve as a dict key
when its fields are hashable. [Slots](https://docs.python.org/3/reference/datamodel.html#slots) reject writes to
misspelled attributes and make each instance smaller. Both options need Python 3.10 or later, so nothing is reported
when the target version is older.

[SMP114](SMP114.html) asks for `kw_only=True` on the same decorator.

## Example

```python
@dataclass
class Release:
    version: str
```

Use instead:

```python
@dataclass(frozen=True, slots=True)
class Release:
    version: str
```

## Fix safety

The fix sets the missing options to `True` on the decorator. It leaves out an option when the files simplint checks show
code that the option breaks:

- `frozen=True` is left out when the class defines `__setattr__` or `__delattr__`, when one of its methods sets or
  deletes an attribute on `self`, or when other code sets or deletes an attribute on an instance known to be of the
  class, such as `result.log = log` after `result = Result(...)`;
- `slots=True` is left out when the class declares `__slots__`, uses
  [`cached_property`](https://docs.python.org/3/library/functools.html#functools.cached_property), or has a subclass
  that does, or when code sets an attribute that is not a field on `self` or on an instance known to be of the class.

When both options are left out, no fix is offered. An object is known to be of the class when it is `self` in a method
of the class, or a name bound to a call of the class or annotated with it, `Optional[...]` and `... | None` included. An
object known to be a mock, such as a name bound to `Mock()`, `MagicMock()`, `create_autospec(...)`, `mocker.Mock()` or
`mocker.patch(...)`, a `with patch(...) as name` target, or anything reached from one, never counts.

The fix is safe only when nothing else in the checked files might notice the change. It is unsafe when:

- the decorator already sets `frozen` or `slots` to something other than `True`;
- the class has base classes, or any class in the checked files inherits from a class with its name;
- any checked file assigns or deletes an attribute named like one of the fields on an object of unknown class, directly
  or through `setattr`;
- another class with the same name, defined in another module, gets attributes written on its instances or has a
  subclass that uses `cached_property`, as an import simplint cannot follow, such as through a re-export, may name this
  class;
- the class body assigns or deletes an attribute on anything other than `self`, or mentions `super`, `__dict__`,
  `__weakref__`, `__setattr__` or `__delattr__`.

Code outside the checked files, and code that adds an attribute that is not a field to an object simplint cannot tie to
the class, is not detected, and breaks once the fix is applied.
