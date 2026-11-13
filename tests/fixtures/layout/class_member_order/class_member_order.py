from typing import overload


class CtorPlain:
    def load(self) -> int:
        return 1

    def __init__(self) -> None:
        self.value = 1


class CtorOverloaded:
    def load(self) -> int:
        return 1

    @overload
    def __init__(self, value: int) -> None: ...

    @overload
    def __init__(self, value: str) -> None: ...

    def __init__(self, value: int | str) -> None:
        self.value = value


class CtorOrdered:
    SIZE = 3

    def __init__(self) -> None:
        self.size = self.SIZE

    def load(self) -> int:
        return self.size


class CtorNested:
    class Config:
        pass

    def load(self) -> int:
        return 1

    def __post_init__(self) -> None:
        self.ready = True

    def __init_subclass__(cls) -> None:
        pass


class CtorCommented:
    def load(self) -> int:
        return 1

    # Reads the separator the caller gives.
    def __init__(self, sep: str = ",") -> None:
        self.sep = sep


class CtorDefaults:
    def load(self) -> int:
        return 1

    LIMIT = 3

    def __init__(self, limit: int = LIMIT) -> None:
        self.limit = limit


class CtorRebound:
    def load(self) -> int:
        return 1

    def __init__(self) -> None:
        self.ready = True

    __init__ = None


class CtorAliased:
    def load(self) -> int:
        return 1

    def __init__(self) -> None:
        self.ready = True

    ALIAS = __init__


class CtorOverloadAfterAlias:
    def load(self) -> int:
        return 1

    Value = int

    @overload
    def __init__(self, value: Value) -> None: ...

    @overload
    def __init__(self, value: str) -> None: ...

    def __init__(self, value: int | str) -> None:
        self.value = value
