[`accumulator-loop`](../src/rules/logic/accumulator_loop.md) reports a loop that only adds to a string, bytes or number
started empty or at `0` the line before, and its unsafe fix builds the value with `str.join()` or `sum()`.
