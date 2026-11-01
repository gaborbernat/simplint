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

The fix sets the missing options to `True` on the decorator. No fix is offered for a class without fields, such as a
test fixture standing for a dataclass with the default options, or for a class the module creates inside
`with pytest.raises(...)` or `with self.assertRaises(...)`, as the test expects code using the instance to fail, such as
`{Key("a"): True}` raising because the class is unhashable, and either option can change that. Otherwise it leaves out
an option when the files simplint checks show code that the option breaks:

- `frozen=True` is left out when the class has a base class other than `Generic[...]`, whose methods may set attributes
  and which a frozen dataclass cannot inherit from unless it is frozen too, when any class in the checked files inherits
  from a class with its name, when the class defines `__setattr__` or `__delattr__`, when one of its methods sets or
  deletes an attribute on `self`, or when other code sets or deletes an attribute on an instance known to be of the
  class, such as `result.log = log` after `result = Result(...)`, or sets or deletes an attribute named like one of the
  fields or `ClassVar` attributes on an object of unknown class, such as `envs[name].active = True`, directly or through
  `setattr`;
- `slots=True` is left out when the class declares `__slots__`, uses
  [`cached_property`](https://docs.python.org/3/library/functools.html#functools.cached_property), or has a subclass
  that does, or a subclass with several bases, such as `class Context(FileContext, threading.local)`, whose layouts
  conflict with a slotted one, or when code sets an attribute that is not a field on `self` or on an instance known to
  be of the class, a `ClassVar` or `InitVar` attribute included, as a slotted instance has no `__dict__` to shadow it
  in, or reaches the instance `__dict__`, as `self.__dict__.update(...)` or `vars(config)` do. It is also left out when
  the class stays mutable and any checked file pickles with a protocol that may be 0 or 1, such as
  `pickle.dumps(obj, protocol)` in a loop over every protocol, as those protocols refuse a slotted class without
  `__getstate__`, which a dataclass gets only when frozen; and when the class is frozen and has a subclass whose methods
  store attributes on `self`, as the `__getstate__` a frozen slotted dataclass gets pickles only the fields and drops
  them.

When both options are left out, no fix is offered. An object is known to be of the class when it is `self` in a method
of the class, a name bound to a call of the class or annotated with it, `Optional[...]` and `... | None` included, or an
attribute of `self` the class body annotates with it, such as `self.config` after `config: Config`. An object known to
be a mock, such as a name bound to `Mock()`, `MagicMock()`, `create_autospec(...)`, `mocker.Mock()` or
`mocker.patch(...)`, a `with patch(...) as name` target, or anything reached from one, never counts.

The fix is safe only when nothing else in the checked files might notice the change. It is unsafe when:

- the decorator already sets `frozen` or `slots` to something other than `True`;
- the class has base classes;
- another class with the same name, defined in another module, gets attributes written on its instances or has a
  subclass that a slotted base breaks, as an import simplint cannot follow, such as through a re-export, may name this
  class;
- the class body assigns or deletes an attribute on anything other than `self`, or mentions `super`, `__dict__`,
  `__weakref__`, `__setattr__` or `__delattr__`.

Code outside the checked files, and code that adds an attribute that is not a field to an object simplint cannot tie to
the class, is not detected, and breaks once the fix is applied.
