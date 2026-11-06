`module-variable-case` (SMP102), `module-variable-prefix` (SMP404) and `unexported-public-name` (SMP403) no longer
rename a name that a string of any file in the run spells, such as the `_compat.repr_context` a generated code template
reads or the `"main"` a test looks up with `runpy`. The check used to cover only the file being fixed, so an unsafe fix
could break a test in another file.
