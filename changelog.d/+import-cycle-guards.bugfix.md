[`import-cycle`](../src/rules/imports/import_cycle.md) (SMP308) no longer exempts the `else` of
`sys.version_info[0] == 3`, which no supported Python runs. It counts the import under a `TYPE_CHECKING` bound to a true
constant, and under a public false constant that a star import rebinds, which the module runs and so closes the cycle.
