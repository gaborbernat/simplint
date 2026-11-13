[`catastrophic-regex`](../src/rules/logic/catastrophic_regex.md) leaves a nested repeat alone when only parts that can
match nothing follow it and the call need not reach the end of the string, since nothing then backtracks into it.
