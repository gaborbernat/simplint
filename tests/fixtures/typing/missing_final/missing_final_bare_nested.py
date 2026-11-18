from typing import Final

TIMEOUT: Final = 1


class Config:
    RATE: Final = 2
    NAMES: Final[list[str]] = []


class Builder:
    def build(self, x: str) -> None:
        sbom: Final = "virtualenv.pyz.cdx.json"
        min_parts: Final = 5
        dist_info: Final = self.find_dist_info(x)
        LOCAL = 3
        print(sbom, min_parts, dist_info, LOCAL)

    def find_dist_info(self, x: str) -> str:
        return x


def outer() -> None:
    def inner() -> None:
        ceiling: Final = 1.5
        print(ceiling)


def blocks(flag: bool, items: list[str]) -> None:
    if flag:
        ratio: Final = 0.5
    for _ in items:
        label: Final = b"x"
    while flag:
        enabled: Final = True
        break
    with open("names.txt") as handle:
        name: Final = "x"
    try:
        size: Final = 3
    except OSError:
        size = 0
    match items:
        case [first]:
            kind: Final = "k"
        case _:
            kind = "other"
    print(ratio, label, enabled, handle, name, size, first, kind)
