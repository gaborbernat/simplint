The [`mock-without-spec`](../src/rules/testing/mock_without_spec.md) rule now reports a `spec`, `spec_set` or positional
spec and a `wraps` argument, since they do not check call signatures, and asks for `autospec`. A spec for a class the
module defines stays accepted when the test reads an instance attribute or a property that autospeccing cannot see
through the mock. A `patch(..., spec=True)` gets an unsafe fix to `autospec=True`.
