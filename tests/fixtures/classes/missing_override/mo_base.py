class MoBase:
    def run(self) -> int:
        return 1

    def stop(self) -> int:
        return 0

    @property
    def size(self) -> int:
        return 1

    def __repr__(self) -> str:
        return "MoBase"


class MoMiddle(MoBase):
    def pause(self) -> int:
        return 2
