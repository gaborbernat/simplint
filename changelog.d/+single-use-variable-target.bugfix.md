`single-use-variable` (SMP504) no longer inlines a variable where the target Python version rejects the result or the
meaning changes: a value with a backslash before Python 3.12, a conditional read, an annotation, a `type` statement, an
`except` type, a `match` case, or a decorator before Python 3.9. Before Python 3.12, a value whose strings use the
f-string's quote takes the other quote in those strings. A variable read in an f-string `=` debug field is inlined with
the printed text unchanged. A value that starts with `{` gets a space inside its field, so the field does not read as an
escaped brace.
