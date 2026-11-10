[`rst-unknown-param`](../src/rules/docstrings/rst_unknown_param.md) (SMP917) no longer reports `:param x:` of an attrs
field `_x`, which attrs takes as `x`. [`rst-missing-param`](../src/rules/docstrings/rst_missing_param.md) (SMP916) asks
for `x` in that case, and takes the `alias` of a field as its name.
