[`test-class`](../src/rules/testing/test_class.md) (SMP801) no longer flattens two classes of one file onto the same
function name. The first class keeps its fix and a later class with that name is reported without one; before, the
second function shadowed the first and one test was lost.
