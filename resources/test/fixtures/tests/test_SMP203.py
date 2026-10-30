from typing import Final
from unittest.mock import MagicMock, create_autospec


OPTIONS: Final[dict[str, int]] = {}


def run() -> int:
    return 1


def test_mocks() -> None:
    first = MagicMock(spec=run)
    first.return_value = 3
    first.side_effect = None
    second = create_autospec(run)
    second.return_value = 4
    second.name = "special"
    third = MagicMock(spec=run, return_value=1)
    third.total = third
    fourth = MagicMock(spec=run,)
    fourth.value = 2  # explained
    fifth = MagicMock(spec=run,)
    fifth.value = 2
    assert first() + second() + third() + fourth() + fifth()


def test_more() -> None:
    plain = run()
    plain.value = 1
    sixth = MagicMock()
    sixth.value = 3
    seventh = MagicMock(spec=run, return_value=1)
    seventh.return_value = 2
    seventh.side_effect = None
    eighth = MagicMock(spec=run, **OPTIONS)
    eighth.return_value = 2
    ninth = MagicMock(spec=run, return_value=1)
    ninth.return_value = 5
    assert plain and sixth and seventh and eighth and ninth
