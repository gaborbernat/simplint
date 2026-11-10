[`test-only-code`](../src/rules/api/test_only_code.md) no longer reports the functions of a module that production code
passes on as a value, as `codec = fast` or a default argument does, since the code calls them through that value.
