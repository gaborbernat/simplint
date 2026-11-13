[`too-many-attributes`](../src/rules/classes/too_many_attributes.md) and the rules that tell class attributes from
instance ones read `__init_subclass__` and `__class_getitem__` as class methods, which Python makes them without a
decorator.
