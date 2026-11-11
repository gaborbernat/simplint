[`dataclass-slots`](../src/rules/classes/dataclass_slots.md) no longer offers `slots=True` in a module that uses a
`weakref.WeakSet`, `WeakKeyDictionary`, `WeakValueDictionary` or `WeakMethod`, which hold their items through
`weakref.ref`.
