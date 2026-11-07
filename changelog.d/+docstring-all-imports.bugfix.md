`undocumented-public-api` (SMP903), `function-docstring` (SMP901) and `test-only-code` (SMP408) no longer count a name
that another module imports from a library's public module when the module's `__all__` leaves the name out, or when the
module supplies names through a module-level `__getattr__`.
