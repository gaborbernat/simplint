def test_parenthesized_typo(mock):
    assert (mock.called_once_with(1))
