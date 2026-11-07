`module-variable-case` (SMP102) and `module-variable-prefix` (SMP404) no longer report an annotation without a value
under `if TYPE_CHECKING:`. Such a declaration types a name that a module's PEP 562 `__getattr__` serves at runtime.
Renaming it broke the type checker's view of `module.log`, while `__getattr__` keeps serving it at runtime. An assigned
name in the same block is still checked.
