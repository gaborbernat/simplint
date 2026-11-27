[`dataclass-slots`](../src/rules/classes/dataclass_slots.md) (SMP702) no longer reports a dataclass that uses
`functools.cached_property`, which needs the instance `__dict__` that `slots=True` removes. A cached property on a
subclass no longer withholds the fix, since the subclass keeps its own `__dict__`.
