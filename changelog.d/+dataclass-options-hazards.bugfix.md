`dataclass-options` (SMP111) no longer freezes a dataclass when a field name is written on an object simplint cannot
type, such as `envs[name].active = True`, or when the class has a base class other than `Generic[...]` or a subclass. It
no longer adds `slots=True` when a subclass has several bases, such as `threading.local`, whose layout conflicts.
