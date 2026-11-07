`module-variable-case` (SMP102) and `module-variable-prefix` (SMP404) no longer report a name that one module-level
`if`, `try` or `with` block binds and reads, such as `msg` in a version check before a `raise`. A name read outside the
block, or by another file, is still reported.
