[loop-in-test](../src/rules/testing/loop_in_test.md) follows each chain of local names that a test's table is built from
once instead of once per path through it, so eight chained names with twelve references each take under 0.01 seconds
instead of 16.4. [unused-definition](../src/rules/api/unused_definition.md) reads the words of a file's doctests once
instead of rescanning every docstring for each definition it checks, so 16,000 documented functions take 0.06 seconds
instead of 3.1. [suppression-without-reason](../src/rules/comments/suppression_without_reason.md) looks up the reason of
each suppression in one map of the file's comment lines instead of scanning every comment run for it, so 64,000
suppressions take 0.05 seconds instead of 4.8. [definition-before-use](../src/rules/layout/definition_before_use.md) and
[class-member-order](../src/rules/layout/class_member_order.md) compare each class statement with the others by name
instead of with every statement, and share one build of each class order, so a class of 64,000 attributes takes 0.05
seconds under SMP201 instead of 2.8 and 0.06 under SMP203 instead of 2.6.
[definition-before-use](../src/rules/layout/definition_before_use.md) finds the annotations that a decorator reads with
a binary search over the sorted annotation ranges instead of a scan of all of them, so 32,000 decorated functions take
0.12 seconds instead of 4.2. [single-caller-helper](../src/rules/logic/single_caller_helper.md) finds the caller of each
helper in a map of the file's functions instead of scanning them, and checks the size of a caller before walking it for
loops, so 64,000 helpers take 0.31 seconds instead of 2.9 and a caller with 32,000 calls to helpers takes 0.14 seconds
instead of 8.8. [walrus-candidate](../src/rules/logic/walrus_candidate.md) reads the names a function declares `global`
or `nonlocal` once per function instead of once per assignment it checks, so 32,000 assignment and `if` pairs in one
function take 0.45 seconds instead of 4.7.
