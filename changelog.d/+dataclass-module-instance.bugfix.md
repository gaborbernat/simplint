[`dataclass-frozen`](../src/rules/classes/dataclass_frozen.md) and
[`dataclass-slots`](../src/rules/classes/dataclass_slots.md) now see an instance built as `models.Release(...)`, or
annotated `models.Release`, as an instance of the class in that module, so a write to it withholds the fix.
