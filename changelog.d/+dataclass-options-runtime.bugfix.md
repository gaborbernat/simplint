`dataclass-options` (SMP111) now sees writes on instances of classes defined in a package's `__init__.py`, and no longer
adds `slots=True` to a class that writes a `ClassVar` through an instance, reaches its `__dict__`, is pickled with
protocol 0 or 1 while mutable, or is frozen with subclasses that store attributes. It offers no fix for a class without
fields or one a test creates inside `pytest.raises`. `dataclass-kw-only` (SMP114) treats `module.Name(...)` positional
calls on a module no import binds as uses it cannot rewrite.
