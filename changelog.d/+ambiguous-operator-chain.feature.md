[`ambiguous-operator-chain`](../src/rules/logic/ambiguous_operator_chain.md) reports comparison chains that mix operator
kinds, such as `key == value in table` or `a != b != c`, and a bitwise operation in the middle of a chain, such as
`a > 1 & b < 2`, which binds tighter than the comparisons around it.
