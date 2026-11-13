[`test-process-state`](../src/rules/testing/test_process_state.md) reports a test, fixture or test class method that
changes an environment variable, the working directory, `sys.path`, `sys.argv`, `sys.modules` or a module attribute by
hand, which leaks into the tests that run after it; `monkeypatch` makes the same change and undoes it.
