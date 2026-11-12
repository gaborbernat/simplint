from dataclasses import dataclass


@dataclass
class Release:
    version: str

    def label(self) -> str:
        return super().__repr__()


@dataclass
class Nested:
    version: str

    def label(self) -> str:
        def describe(item: "Nested") -> str:
            return super().__repr__()

        return describe(self)


@dataclass
class Lambda:
    version: str

    def label(self) -> str:
        return (lambda item: super().__repr__())(self)


@dataclass
class Kind:
    version: str

    def kind(self) -> type:
        return __class__


@dataclass
class Explicit:
    version: str

    def label(self) -> str:
        return super(Explicit, self).__repr__()


@dataclass
class Attribute:
    version: str

    def kind(self) -> type:
        return self.__class__
