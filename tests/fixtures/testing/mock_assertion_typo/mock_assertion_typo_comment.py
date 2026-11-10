def test_commented_typo(mock):
    assert (  # the writer must be reached exactly once
        mock.called_once_with(1)
    )


def test_commented_argument(mock):
    assert (mock.called_once_with(  # the argument is kept
        1))


def test_trailing_comment(mock):
    assert mock.called_once_with(1)  # not part of the assertion
