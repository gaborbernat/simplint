A name a function binds to modules, as `mod = pytest.importorskip(...)` does, reads the names of those modules only, so
`module-variable-prefix` (SMP404) no longer offers to rename a name a test reads off a module it loads. A local variable
that shares its name with a module-level one no longer makes that name count as read in other files, which reported a
variable of an `if` block that only the block reads as `module-variable-case` (SMP102).
