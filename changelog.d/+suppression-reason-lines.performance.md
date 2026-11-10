[`suppression-without-reason`](../src/rules/comments/suppression_without_reason.md) looks up the reason of each
suppression in one map of the file's comment lines instead of scanning every comment run for it, so 64,000 suppressions
take 0.05 seconds instead of 4.8.
