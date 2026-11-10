[`test-only-code`](../src/rules/api/test_only_code.md) (SMP408) also keeps the functions of a module that production
code reaches as `app.fast`, that a class body imports, or that a function body imports and uses as a value, since the
code calls them through that module. A function that only calls the module's functions does not keep the others.
