[rst-missing-param](../src/rules/docstrings/rst_missing_param.md) and
[rst-attribute-kind](../src/rules/docstrings/rst_attribute_kind.md) read a quoted annotation such as `"ClassVar[int]"`
as the `ClassVar` it spells, so a class variable declared that way needs no `:param:` field, and an `:ivar:` field for
it is reported.
