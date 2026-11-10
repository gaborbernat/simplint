[`mock-assertion-typo`](../src/rules/testing/mock_assertion_typo.md) fixes `assert (mock.called_once_with(1))` to
`mock.assert_called_once_with(1)`. The fix used to keep the closing parenthesis, which does not parse, so the fix was
rejected.
