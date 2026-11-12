[`dataclass-slots`](../src/rules/classes/dataclass_slots.md) no longer reports a dataclass whose body uses zero-argument
`super()` or `__class__` when the target version is before 3.14. Slotting such a class made `super()` raise `TypeError`
until CPython fixed it in 3.14 (gh-90562), and 3.13 has the fix only from 3.13.14, so 3.13 counts as unfixed.
