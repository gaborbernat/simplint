[`failed-swap`](../src/rules/logic/failed_swap.md) reports `a = b` followed by `b = a`, which leaves both holding the
old `b`, with an unsafe fix to `a, b = b, a`, and augmented assignments such as `total += total + delta` that apply
their target twice.
