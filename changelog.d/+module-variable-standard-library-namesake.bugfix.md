`module-variable-case` (SMP102) and `module-variable-prefix` (SMP404) keep the names of a file named after a standard
library module, such as `typing_extensions.py`, when another file of the run reads them off that module. The import
resolves to the standard library, so the run cannot tell which file its reads reach.
