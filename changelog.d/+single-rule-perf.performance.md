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
function take 0.45 seconds instead of 4.7. [early-assignment](../src/rules/logic/early_assignment.md) searches the
string tokens and the statements of a block by position instead of scanning them for each constant, so 32,000 constants
assigned above their uses take 0.05 seconds instead of 1.8.
[up-unittest-enter-context](../src/rules/testing/up_unittest_enter_context.md) counts the names of each test method once
instead of scanning the tokens of the whole file for every patch it checks, so 16,000 manual patch sequences take 0.15
seconds instead of 3.3. [mock-assertion-typo](../src/rules/testing/mock_assertion_typo.md) checks each misspelled name
against a set of the file's function names instead of scanning every function for each one, so 32,000 misspelled calls
take 0.14 seconds instead of 0.9. [rst-param-order](../src/rules/docstrings/rst_param_order.md) and
[rst-type-field](../src/rules/docstrings/rst_type_field.md) look up the parameter of each field in a map instead of
scanning the signature, and [rst-unknown-attribute](../src/rules/docstrings/rst_unknown_attribute.md) finds each
attribute's earlier definition by name, so 32,000 parameters or attributes take 0.04 and 0.06 seconds instead of 0.46
and 0.57.
