class CtorEager:
    def load(self) -> int:
        return 1

    Limit = int

    def __init__(self, limit: Limit) -> None:
        self.limit = limit
