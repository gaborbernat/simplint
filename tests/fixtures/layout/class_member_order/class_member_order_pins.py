class Splits:
    def run(self) -> int:
        return 1

    @property
    def size(self) -> int:
        return self.make()

    def make(self) -> int:
        return 1

    SIZE = make

    @size.setter
    def size(self, value: int) -> None:
        pass

    def __init__(self) -> None:
        self.value = 0


class Crossing:
    def run(self) -> int:
        return self._check()

    NAME = "x"

    def _check(self, name: str = NAME) -> int:
        return 1

    def __init__(self) -> None:
        self.value = 0
