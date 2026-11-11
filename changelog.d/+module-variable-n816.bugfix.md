[`module-variable-case`](../src/rules/naming/module_variable_case.md) (SMP102) leaves a `mixedCase` name to Ruff's N816
when it has no rename, whether or not N816 is selected. The name stays reported when it can be renamed, and N816 reads
the stores of the module scope, so a `global` declaration in a function no longer hides the module store that N816
reports.
