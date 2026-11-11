[`unused-export`](../src/rules/api/unused_export.md) (SMP402) no longer reports a name of a module that production code
loads as a value, as `CONFIG = settings` does, or a submodule that a star import binds through its package's `__all__`.
A test that passes the module on exempts nothing.
