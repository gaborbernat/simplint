[`unused-export`](../src/rules/api/unused_export.md) (SMP402) no longer turns off for the whole run when a file reads
`__all__` off a module it cannot name, such as `sys.modules["json"]`. Only an import of `__all__` or a read of it off a
module of the run exempts a module's `__all__`.
