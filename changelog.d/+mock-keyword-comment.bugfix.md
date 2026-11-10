[`mock-attribute-assignment`](../src/rules/testing/mock_attribute_assignment.md) offers its fix when a comment follows
the trailing comma of the constructor call. The fix used to add a second comma before the new keyword, which does not
parse, so the fix was rejected.
