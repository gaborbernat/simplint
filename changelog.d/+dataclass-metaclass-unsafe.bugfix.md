[`dataclass-slots`](../src/rules/classes/dataclass_slots.md) and
[`dataclass-frozen`](../src/rules/classes/dataclass_frozen.md) make their fix unsafe for a class with a keyword such as
`metaclass=`, since the slotted copy runs the metaclass twice.
