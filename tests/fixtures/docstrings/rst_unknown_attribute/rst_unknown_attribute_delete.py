class Slot:
    """A slot.

    :ivar kept: Still set.
    :ivar removed: Deleted in a method.
    :ivar patched: Set through `object.__setattr__`.
    :ivar nested: Set in a nested function.
    """

    def __init__(self) -> None:
        self.kept = 1
        del self.removed

    def freeze(self) -> None:
        object.__setattr__(self, "patched", 2)

        def helper() -> None:
            self.nested = 3

        helper()
