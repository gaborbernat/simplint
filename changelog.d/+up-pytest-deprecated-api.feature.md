[`up-pytest-deprecated-api`](../src/rules/testing/up_pytest_deprecated_api.md) reports pytest APIs that pytest 8 and 9
removed or no longer honor: nose-style `setup` and `teardown` hooks, marks on fixtures, the `tmpdir`, `tmpdir_factory`
and `testdir` fixtures, `msg=` to `pytest.fail`, `pytest.skip` and `pytest.exit`, `pytest.raises(message=...)` and
`pytest.warns(None)`. It renames `msg` to `reason`, and, as an unsafe fix, a nose hook to its pytest name.
