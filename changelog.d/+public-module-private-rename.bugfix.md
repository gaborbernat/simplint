`unexported-public-name` (SMP403) and `module-variable-prefix` (SMP404) still report a name missing from `__all__` in a
public module of a library, but no longer offer to rename it private, since users import those names. The report asks
for the name in `__all__` or a rename by hand.
