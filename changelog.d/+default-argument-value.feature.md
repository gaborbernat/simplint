[`default-argument-value`](../src/rules/logic/default_argument_value.md) reports an argument that passes a parameter of
a function in the same module the literal its default already gives, as a keyword or as the last positional argument.
Such an argument pins the call to today's default and suggests the value matters there. The safe fix removes it. A
parameter every call passes is left to [`unused-default`](../src/rules/logic/unused_default.md).
