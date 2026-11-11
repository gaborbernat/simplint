[`string-concatenation`](../src/rules/logic/string_concatenation.md) reads the bindings of a scope in one walk for all
of its names, instead of one walk per name. A module of 16,000 annotated classes, each used by a function, takes 0.16
seconds under SMP503 instead of 2.9.
