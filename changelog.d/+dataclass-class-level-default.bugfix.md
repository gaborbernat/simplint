[`dataclass-slots`](../src/rules/classes/dataclass_slots.md) no longer offers `slots=True` for a class whose field
default is read or assigned through the class, as `Config.timeout`, or through `cls` in a classmethod, since slots
replace that class attribute with a descriptor.
