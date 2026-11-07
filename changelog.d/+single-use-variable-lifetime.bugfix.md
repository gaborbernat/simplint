`single-use-variable` (SMP504) no longer inlines a value into the `next`, `anext`, `send` or `close` call that reads it.
Inlined, the call became the only holder of a generator, so its `finally` block ran as soon as the call returned.
