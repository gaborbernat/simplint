[`single-use-variable`](../src/rules/logic/single_use_variable.md) (SMP504) no longer inlines a string into a field of a
nested f-string before Python 3.12 when the string holds the quote of an enclosing f-string. The fix had made the file
fail to parse, which dropped every other fix in it.
