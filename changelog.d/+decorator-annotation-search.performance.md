[`definition-before-use`](../src/rules/layout/definition_before_use.md) finds the annotations that a decorator reads
with a binary search over the sorted annotation ranges instead of a scan of all of them, so 32,000 decorated functions
take 0.12 seconds instead of 4.2.
