`conditional-in-test` (SMP803) no longer reports a conditional expression that is the item of a comprehension, such as
`[tag if tag else "-" for tag in tags]`, as it already did for `if` clauses in comprehensions. A conditional nested
inside such an item is still reported.
