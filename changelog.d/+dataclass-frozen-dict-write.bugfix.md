`dataclass-frozen` (SMP701) no longer offers `frozen=True` for a class whose instances get a `__dict__` store, as
`x.__dict__ = ...`, `del x.__dict__` or `setattr(x, "__dict__", ...)`, since a frozen instance raises
`FrozenInstanceError` on it and the fix broke the code it changed. Reading `__dict__`, as `vars(x)` does, keeps the fix.
