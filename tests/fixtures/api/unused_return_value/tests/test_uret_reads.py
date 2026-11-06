from _unused_return_value import ret_tests_only, ret_tests_read


def test_tests_only() -> None:
    assert ret_tests_only(1) == 4


def test_tests_read() -> None:
    assert ret_tests_read(1) == 6
