[`up-type-checking-lazy-import`](../src/rules/imports/up_type_checking_lazy_import.md) no longer adds a second `lazy` to
an import that is `lazy` already, which produced a syntax error and reverted the fixes of the file.
