The fixes of `module-variable-case` (SMP102) and `module-variable-prefix` (SMP404) no longer rename a name that a public
module of a library offers or lists in `__all__`, since its users import it.
