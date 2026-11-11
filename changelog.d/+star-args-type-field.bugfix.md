[`rst-type-field`](../src/rules/docstrings/rst_type_field.md) (SMP912) keeps the stars of `*args` and `**kwargs` when
its fix drops the type from `:param int \*args:`, which before wrote `:param args:`.
