import unittest
from unittest.mock import MagicMock, call

import pytest


def check(mock: MagicMock, numbers: list[int]) -> None:
    mock.called_once_with(1)
    mock.called_with(1)
    mock.called_once()
    mock.not_called()
    mock.has_calls([call(1)])
    mock.any_call(1)
    mock.awaited_once()
    mock.assert_called_ones_with(1)
    mock.asert_called_with(1)
    mock.assrt_called_once()
    mock.assert_calls(1)
    mock.assert_failed()
    mock.asssert_called_once()
    mock.assetr_called_with(1)
    mock.awaited_once_with(1)
    mock.assess_values(1)
    mock.assert_called_onth(1)
    mock.called_wiht(1)
    mock.assert_called_once_with(1)
    mock.assert_called_with(1)
    assert mock.called
    assert mock.call_count == 1
    assert mock.not_called()
    assert mock.called_once_with(1)
    assert mock.awaited_once
    assert not mock.not_called()
    assert mock.not_called(mock.called_with)
    assert mock.called_once_with(1), "called once"
    flag = mock.has_calls
    mock.called_once
    print(flag, mock.called_with(1))
    numbers.assert_equal(1)


def expects_failure(mock: MagicMock, case: unittest.TestCase) -> None:
    with pytest.raises(AttributeError):
        mock.called_once()
    with case.assertRaisesRegex(AttributeError, "called_once"):
        mock.called_once_with()
    with open("handle") as handle:
        mock.called_with(handle)
