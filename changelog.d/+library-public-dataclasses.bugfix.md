`dataclass-slots` (SMP702) and `dataclass-frozen` (SMP701) no longer report the public dataclasses of a library,
including those nested in its public classes, since `slots` and `frozen` change what users can do with an instance.
