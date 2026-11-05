`future-annotations` (SMP301) no longer reports the import when a class annotation reads a name the class body binds,
such as a method named `list` in `def list(self) -> list[str]`. Without the import, Python 3.14 and later evaluate that
annotation in the class namespace, where `list` is the method.
