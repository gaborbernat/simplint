[`all-format`](../src/rules/api/all_format.md) (SMP405) leaves a `+=` on `__all__` alone when a plain assignment keeps
`__all__` a tuple: one that is not a list or tuple literal, is nested in a block, or has a `noqa` for this rule.
Converting the `+=` used to fail with a `TypeError`.
