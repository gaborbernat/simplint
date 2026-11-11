[`dataclass-slots`](../src/rules/classes/dataclass_slots.md) no longer offers `slots=True` for an instance that a
`weakref.WeakSet`, `WeakKeyDictionary`, `WeakValueDictionary` or `WeakMethod` holds, as those hold their items through
`weakref.ref`, wherever the container is used in the checked files.
