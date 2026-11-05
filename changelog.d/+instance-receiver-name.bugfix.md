`dataclass-frozen` (SMP701) and `dataclass-slots` (SMP702) treat whatever a method names its first parameter as the
instance, not only `self`, so attributes set through `def m(this)` count.
