[missing-final](../src/rules/typing/missing_final.md) no longer adds its import above a module docstring that shares its
line with the next statement, which turned `__doc__` into `None`. The report stays, without the fix.
