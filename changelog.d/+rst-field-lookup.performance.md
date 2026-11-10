[`rst-param-order`](../src/rules/docstrings/rst_param_order.md) and
[`rst-type-field`](../src/rules/docstrings/rst_type_field.md) look up the parameter of each field in a map instead of
scanning the signature, and [`rst-unknown-attribute`](../src/rules/docstrings/rst_unknown_attribute.md) finds each
attribute's earlier definition by name, so 32,000 parameters or attributes take 0.04 and 0.06 seconds instead of 0.46
and 0.57.
