`star-import` (SMP309) reports `from module import *` when the project defines `module` and the file uses its names, and
replaces the `*` with the names the file reads from it. The fix is safe when the module lists its exports in `__all__`,
and unsafe otherwise.
