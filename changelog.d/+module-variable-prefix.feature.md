`module-variable-prefix` (SMP404) flags a module variable whose leading underscore does not match its visibility: a name
that no other module imports and `__all__` does not list needs one, and a name `__all__` lists must not have one.
