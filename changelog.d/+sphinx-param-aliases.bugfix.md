[`rst-field-syntax`](../src/rules/docstrings/rst_field_syntax.md) (SMP910) and
[`rst-missing-param`](../src/rules/docstrings/rst_missing_param.md) (SMP916) read `:kwarg:` and `:kwparam:` as spellings
of `:param:`, and `:paramtype:` as one of `:type:`, as Sphinx does, instead of reporting them as unknown fields.
