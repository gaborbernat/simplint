[`unreachable-code`](../src/rules/logic/unreachable_code.md) reports statements that never run: those after a `return`,
`raise`, `break` or `continue`, after an `if` or `try` whose every branch leaves, after a `while True` with no `break`,
and under an `if False:` or `while 0:`. Its unsafe fix deletes them.
