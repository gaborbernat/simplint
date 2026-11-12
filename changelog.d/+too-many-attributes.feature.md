[`too-many-attributes`](../src/rules/classes/too_many_attributes.md) reports a class whose instances hold more
attributes than `classes.max-instance-attributes`, 12 by default, counting what its methods set on `self` and a
dataclass's fields, so related attributes move into classes of their own.
