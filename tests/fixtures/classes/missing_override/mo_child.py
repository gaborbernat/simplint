from typing import overload

import typing_extensions

from mo_base import MoBase, MoMiddle


class MoChild(MoMiddle):
    def run(self) -> int:
        return 3

    @typing_extensions.override
    def stop(self) -> int:
        return 4

    @property
    def size(self) -> int:
        return 5

    @size.setter
    def size(self, value: int) -> None:
        print(value)

    def __repr__(self) -> str:
        return "MoChild"

    def pause(self) -> int:
        return 6

    def own(self) -> int:
        return 7


class MoLocal:
    def run(self) -> int:
        return 8


class MoLocalChild(MoLocal):
    @overload
    def run(self, value: int) -> int: ...

    @overload
    def run(self, value: str) -> str: ...

    def run(self, value: object = None) -> object:
        return value


class MoUnrelated:
    def run(self) -> int:
        return 9


print(MoBase, MoChild, MoLocalChild, MoUnrelated)
