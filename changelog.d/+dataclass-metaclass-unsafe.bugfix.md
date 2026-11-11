[`dataclass-slots`](../src/rules/classes/dataclass_slots.md) makes its fix unsafe for a class with a keyword such as
`metaclass=`, since the slotted copy runs the metaclass twice.
