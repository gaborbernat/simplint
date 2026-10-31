`mock-without-spec` accepts `new=None` and `new=False`, `mock-attribute-assignment` also reports annotated assignments,
and its fix is unsafe for `create_autospec` attributes other than `return_value` and `side_effect`.
