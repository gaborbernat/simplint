[`dataclass-frozen`](../src/rules/classes/dataclass_frozen.md) no longer offers `frozen=True` for a class written
through a parameter annotated `Any`, a protocol, a type alias or a type variable, since none of those names a class of
the run. A field the class body annotates with a class of the run is an instance of that class in its methods, even when
no method assigns it, so [`dataclass-slots`](../src/rules/classes/dataclass_slots.md) sees the writes through it too.
