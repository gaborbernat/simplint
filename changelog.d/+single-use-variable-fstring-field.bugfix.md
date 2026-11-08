`single-use-variable` (SMP504) no longer asks to inline a conditional expression, or a value with a string literal in
it, into an f-string replacement field, where the result reads worse than the two statements it replaces. A string value
on its own is still inlined.
