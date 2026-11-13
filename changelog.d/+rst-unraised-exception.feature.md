[`rst-unraised-exception`](../src/rules/docstrings/rst_unraised_exception.md) reports a `:raises X:` field for an
exception the function can no longer raise: no `raise` in its body or in the same-module functions and methods it calls
names `X` or a subclass, and no operation in them raises it on its own. A body that calls code outside the module, or
does anything else that can raise an arbitrary exception, is left alone. The unsafe fix removes the field.
