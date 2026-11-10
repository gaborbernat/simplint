from unittest.mock import MagicMock


def test_comment_after_trailing_comma() -> None:
    client = MagicMock(
        spec=dict,  # the real type
    )
    client.timeout = 3
    assert client.timeout == 3
