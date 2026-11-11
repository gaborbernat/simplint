[`rst-unknown-attribute`](../src/rules/docstrings/rst_unknown_attribute.md) (SMP921) counts the methods in a class's
`if` and `try` blocks and the loop variables of its body as attributes, so their fields are no longer removed. A loop
variable of the class body is a class variable for
[`rst-missing-attribute`](../src/rules/docstrings/rst_missing_attribute.md) (SMP920) and
[`rst-attribute-kind`](../src/rules/docstrings/rst_attribute_kind.md) (SMP922) too: SMP920 asks to document it, and
SMP922 reports an `:ivar:` for it.
