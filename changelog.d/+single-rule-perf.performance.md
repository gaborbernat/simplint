[loop-in-test](../src/rules/testing/loop_in_test.md) follows each chain of local names that a test's table is built from
once instead of once per path through it, so eight chained names with twelve references each take under 0.01 seconds
instead of 16.4. [unused-definition](../src/rules/api/unused_definition.md) reads the words of a file's doctests once
instead of rescanning every docstring for each definition it checks, so 16,000 documented functions take 0.06 seconds
instead of 3.1. [suppression-without-reason](../src/rules/comments/suppression_without_reason.md) looks up the reason of
each suppression in one map of the file's comment lines instead of scanning every comment run for it, so 64,000
suppressions take 0.05 seconds instead of 4.8.
