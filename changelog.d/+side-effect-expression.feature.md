[`side-effect-expression`](../src/rules/logic/side_effect_expression.md) reports a comprehension used as a statement
only for the calls in it, and `a and f()` or `a or f()` used as a statement. Its fix writes a `for` loop or an `if`.
