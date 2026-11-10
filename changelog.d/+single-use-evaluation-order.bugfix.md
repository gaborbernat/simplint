[single-use-variable](../src/rules/logic/single_use_variable.md) keeps the order of evaluation when it inlines a value:
a fix that moves a call, a lookup or a walrus past the read is now unsafe, and a read in a t-string is reported without
a fix. A string with braces that replaces a field in a format spec, as in `{v:{inner}}`, no longer changes what the
f-string prints.
