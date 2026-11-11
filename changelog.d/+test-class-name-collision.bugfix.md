[`test-class`](../src/rules/testing/test_class.md) (SMP801) no longer flattens onto a function name that another class
or another method of the same class already takes, which shadowed a test. The first class keeps its fix; a later class,
or a class with two methods of one name, is reported without one.
