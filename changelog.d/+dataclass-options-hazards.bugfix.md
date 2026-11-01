`dataclass-options` (SMP111) no longer counts attributes set on mocks, such as `client.uuid = ...` on a `MagicMock()`,
as writes to a dataclass field, and ties a write to the class of its receiver when parsing tells it. When the checked
files show code that an option breaks, the fix leaves that option out: no `frozen=True` for a class whose instances get
attributes assigned, and no `slots=True` for a class that uses `functools.cached_property`, has a subclass that does, or
gets attributes that are not fields. With both options broken it offers no fix. When a fix in one file changes these
writes, as a rename that leaves no other class with the dataclass's name, `--fix` checks the dataclass again in the same
run.
