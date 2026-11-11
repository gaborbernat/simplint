[`long-tuple-return`](../src/rules/logic/long_tuple_return.md) (SMP528) no longer reports a function whose return
annotation is a quoted `"tuple[T, ...]"`. The quoted annotation is read like the unquoted one, which the rule already
exempts.
