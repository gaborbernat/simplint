def _dav_fetch(url: str, retries: int = 3, *, verbose: bool = False, label: str = "x") -> str:
    return f"{url}{retries}{verbose}{label}"


def _dav_always(value: int, scale: int = 1) -> int:
    return value * scale


def _dav_posonly(value: int, scale: int = 2, /) -> int:
    return value * scale


def _dav_negative(value: int, floor: int = -1) -> int:
    return max(value, floor)


def _dav_named(value: object, sentinel: object = None) -> object:
    return value or sentinel


_DAV_SPLAT = {"retries": 3}
_DAV_RESULTS = (
    _dav_fetch("a", 3),
    _dav_fetch("b", retries=3, verbose=False),
    _dav_fetch("c", 4, label="x"),
    _dav_fetch("d"),
    _dav_fetch("e", 3, verbose=True),
    _dav_fetch("f", **_DAV_SPLAT),
    _dav_fetch(
        "g",
        verbose=False,  # keep
    ),
    _dav_always(1, 1),
    _dav_always(2, scale=1),
    _dav_posonly(3, 2),
    _dav_posonly(3),
    _dav_negative(4, -1),
    _dav_negative(4),
    _dav_named(5, None),
    _dav_named(5),
)
