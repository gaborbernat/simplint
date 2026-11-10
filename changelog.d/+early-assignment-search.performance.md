[`early-assignment`](../src/rules/logic/early_assignment.md) searches the string tokens and the statements of a block by
position instead of scanning them for each constant, so 32,000 constants assigned above their uses take 0.05 seconds
instead of 1.8.
