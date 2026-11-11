[`dataclass-frozen`](../src/rules/classes/dataclass_frozen.md) and
[`dataclass-slots`](../src/rules/classes/dataclass_slots.md) keep their fixes for a class whose nested class writes to
its own instances, or whose classmethod writes a class attribute that is not a field. A closure or a parameter that
writes the dataclass's instance from a nested class still makes the fix unsafe.
