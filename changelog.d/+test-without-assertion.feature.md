[`test-without-assertion`](../src/rules/testing/test_without_assertion.md) reports a test with no `assert`, no `raise`
and no call that can fail it, following the module's own helpers, so it passes for any code that does not raise. A test
whose name, docstring or comments say it checks for crashes or errors is left alone.
