`split-composite-assert` (SMP807) flags consecutive asserts that compare fields of one object one by one, and its unsafe
fix joins them into one assert over a tuple.
