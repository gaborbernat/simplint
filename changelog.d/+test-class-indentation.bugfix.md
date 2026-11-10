[`test-class`](../src/rules/testing/test_class.md) dedents each method by the indentation of its class body rather than
the file's first indentation. A two-space function above a four-space `class TestThing` used to keep the method nested
in that function, where pytest does not collect it.
