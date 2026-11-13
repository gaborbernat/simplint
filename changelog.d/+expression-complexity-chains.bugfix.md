[`expression-complexity`](../src/rules/logic/expression_complexity.md) counts a chain of calls and subscripts on one
receiver once, as `query.filter(a).order_by(b)` and `data["a"]["b"]` read left to right instead of nesting.
