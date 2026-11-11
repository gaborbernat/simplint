[`single-use-variable`](../src/rules/logic/single_use_variable.md) (SMP504) no longer inlines a value into a `=` debug
field of a raw f-string whose debug text holds the string's quote. The fix wrote the quote with a backslash, which a raw
f-string keeps, so the printed text changed.
