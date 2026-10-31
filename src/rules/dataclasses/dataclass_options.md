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

The fix sets both options to `True` on the decorator. It is safe only when nothing in the files simplint checks can
notice the change. The fix is unsafe when:

- the decorator already sets `frozen` or `slots` to something other than `True`;
- the class has base classes, or any class in the checked files inherits from a class with its name;
- any checked file assigns or deletes an attribute named like one of the fields, directly or through `setattr`;
- the class body assigns or deletes any attribute, or defines or mentions `super`,
  [`cached_property`](https://docs.python.org/3/library/functools.html#functools.cached_property), `__dict__`,
  `__weakref__`, `__slots__`, `__setattr__` or `__delattr__`.

Code outside the class that adds an attribute that is not a field is not detected, and breaks once the fix is applied.
