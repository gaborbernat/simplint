class ReceiverRegistry:
    def _reset(self) -> None:
        pass

    @classmethod
    def _make(cls) -> "ReceiverRegistry":
        return cls()

    @classmethod
    def load(klass) -> "ReceiverRegistry":
        return klass._make()

    @classmethod
    def load_called(klass) -> "ReceiverRegistry":
        return klass()._restore()

    @classmethod
    def _seed(cls) -> "ReceiverRegistry":
        return cls()

    @classmethod
    def load_local(klass) -> "ReceiverRegistry":
        built = klass()
        return built._seed()

    @classmethod
    def _restore(cls) -> "ReceiverRegistry":
        return cls()

    @classmethod
    def _audit(cls) -> None:
        pass

    def _prime(self) -> None:
        pass

    def __new__(cls) -> "ReceiverRegistry":
        made = super().__new__(cls)
        made._prime()
        return made

    def _unprimed(self) -> None:
        pass

    @classmethod
    def blank(cls) -> "ReceiverRegistry":
        made = super().__new__(object)
        made._unprimed()
        return cls()

    @staticmethod
    def unrelated(klass) -> None:
        klass._audit()

    def copy(this) -> "ReceiverRegistry":
        return this._restore()

    def reset(self) -> None:
        def clear() -> None:
            self._reset()

        clear()

    @classmethod
    def create(klass) -> list["ReceiverRegistry"]:
        return list(klass())

    def _lazy(self) -> int:
        return 1

    lazy = property(lambda this: this._lazy())
