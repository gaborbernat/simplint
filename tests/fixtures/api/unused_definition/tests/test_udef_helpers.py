from _unused_definition_used import Recorded, used_by_tests


def udef_helper_unused() -> int:
    return 2


def test_used() -> None:
    assert used_by_tests() == 1
    assert Recorded()
