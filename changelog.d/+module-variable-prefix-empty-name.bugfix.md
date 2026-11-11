[`module-variable-prefix`](../src/rules/api/module_variable_prefix.md) no longer asks to rename `_` to an empty name or
`_class` to the keyword `class`. The fix for those names used to be rejected as a syntax error, and a debug build
panicked.
