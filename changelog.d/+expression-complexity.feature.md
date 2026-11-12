[`expression-complexity`](../src/rules/logic/expression_complexity.md) reports an expression nested deeper than
`logic.max-expression-complexity`, 8 by default, where each operator, comparison, call, subscript, lambda and
comprehension adds a level, so the reader names its parts instead of holding every partial result in mind.
