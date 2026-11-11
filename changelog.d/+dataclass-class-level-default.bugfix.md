[`dataclass-slots`](../src/rules/classes/dataclass_slots.md) no longer offers `slots=True` for a class whose field
default is read or assigned through the class, as `Config.timeout`, through an import alias, a subclass or `cls` in a
classmethod, or through `type(obj)`, `getattr` and `setattr` simplint cannot tie to one class, since slots replace that
class attribute with a descriptor.
