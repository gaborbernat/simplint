from typing import overload as ov


@ov
def pick(value: int) -> int: ...
@ov
def pick(value: str) -> str: ...
def pick(value: int | str) -> int | str:
    return value


class Client:
    def send(self) -> None:
        pass

    @property
    def size(self) -> int:
        return 0


__all__ = ["Client", "pick"]
