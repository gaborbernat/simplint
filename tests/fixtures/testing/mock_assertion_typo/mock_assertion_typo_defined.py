from unittest.mock import MagicMock


class Spec:
    def has_awaits(self) -> None:
        self.awaited_with = True


def defined_here(mock: MagicMock) -> None:
    mock.has_awaits()
    assert mock.awaited_with


def assigned_here(mock: MagicMock) -> None:
    mock.any_await = 1
    mock.any_await()
