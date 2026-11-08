`module-attribute-import` (SMP304) no longer reports a read through an alias of the project's own top-level package,
such as `pd.NaT` for `import pandas as pd`. Reads through its submodules, and through an alias equal to the package
name, are still reported.
