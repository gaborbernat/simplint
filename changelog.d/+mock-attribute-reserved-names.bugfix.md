[`mock-attribute-assignment`](../src/rules/testing/mock_attribute_assignment.md) (SMP805) no longer passes `cls`, `self`
or the private constructor arguments, such as `_new_name`, as keywords. The fix used to make `cls` and `self` raise
`TypeError` and configure the mock for the others, instead of setting the attribute; the run of assignments now stops at
them.
