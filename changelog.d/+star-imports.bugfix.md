Names a `from module import *` can reach count as used: renames to `_`-names by `module-variable-case` (SMP102),
`unexported-public-name` (SMP103), `single-user-constant` (SMP050) and other rules leave them alone, a name imported
from a package counts in the module its `__init__.py` star-imports it from, and `missing-from-all` (SMP113) no longer
creates an `__all__` that would narrow what `import *` takes. A star import that resolves to no file of the run keeps
the names of every file named after its module.
