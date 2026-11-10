[rst-returns](../src/rules/docstrings/rst_returns.md) no longer treats a function as a generator when a lambda in its
body yields, so its `:returns:` field is checked as a return field.
