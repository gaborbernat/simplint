from typing import Final

_TIMEOUT: Final[int] = 30
_RETRIES = 3
_SHARED = 1
# cached: rebuilding the table on every call is too slow
_TABLE = {"a": 1}
PUBLIC = 5


def connect() -> int:
    return _TIMEOUT


def retry() -> int:
    return _RETRIES + _RETRIES + _TABLE["a"]


def first() -> int:
    return _SHARED


def second() -> int:
    return _SHARED + PUBLIC


_PATTERN = "x"
_MULTILINE = (
    "a"
    "b"
)


def loop(items: list[str]) -> list[str]:
    return [item for item in items if item == _PATTERN]


def multi() -> str:
    return _MULTILINE


_REPEAT = 2
_LOOPED = "z"


def twice(items: list[str]) -> list[str]:
    return [item for item in items if item == _LOOPED][:_REPEAT] * _REPEAT

_LAMBDA_ONLY = 3
HANDLER = lambda: _LAMBDA_ONLY


_WARNED: set[str] = set()
_CACHE = {}
_LOG = []
_STATE = Namespace()


def warn_once(message: str) -> None:
    if message not in _WARNED:
        _WARNED.add(message)
    _CACHE[message] = True
    del _CACHE[message]
    _LOG.append(message)
    _STATE.last = message


_LOCK = Lock()
_COUNTER = count()
_LIMIT = 30  # seconds before giving up
_SEEN_TYPES: Final[dict[str, int]] = {}
_NAMES: Final = ("a", "b")


def guarded() -> int:
    with _LOCK:
        return next(_COUNTER) + _LIMIT


def lookup(key: str) -> int:
    return _SEEN_TYPES.get(key, 0) + len(_SEEN_TYPES) + len(_NAMES) + len(_NAMES)


_FULL = ".".join(str(part) for part in VERSION)
_PREFIX = PREFIX_BASE


def describe() -> str:
    return f"version {_FULL} at {_PREFIX}"


class _Empty:
    pass


_IGNORED = frozenset(dir(_Empty))
del _Empty


def ignored() -> frozenset:
    return _IGNORED


_OPERATORS = {"==": operator.eq}


def compare(operator: str) -> object:
    return _OPERATORS[operator]


_LIMITS = (1, 2)


def bounds() -> int:
    return _LIMITS[0] + _LIMITS[1]


_SIZES: dict[str, int] = {}


def size(key: str) -> int:
    return _SIZES.get(key, 0) + len(_SIZES)


_ERRORS = (RuntimeError,) if len(__name__) > 3 else ()


def guarded() -> None:
    try:
        print()
    except (ValueError, *_ERRORS):
        pass


_5_SMOOTH = (1, 2, 3)
_CLASS = (4, 5, 6)


def smooth() -> int:
    return _5_SMOOTH[0] + _5_SMOOTH[1] + _CLASS[0] + _CLASS[1]


_CHOICES = (
    "first",
    "second",
)


def choose(**options: tuple[str, ...]) -> None:
    print(options)


def pick() -> None:
    choose(choices=_CHOICES)


_FACTOR = 3


def scaled() -> int:
    base = 1; doubled = _FACTOR * _FACTOR
    return base + doubled


_WRAPPED: Final[int] = (
    3  # the trailing comment `ruff format` moved inside the parentheses
)


def wrapped() -> int:
    return _WRAPPED


_ROOT = "https://example.com/"
_HOST = "https://example.org/"


class Links:
    def render(self, parts: list[str]) -> str: return "\n".join(f"{_ROOT + part}" for part in parts)

    def prefixed(self, parts: list[str]) -> list[str]: """Prefix each part."""; prefix = "/"; return [_HOST + prefix + part for part in parts]


_RESOURCE_PATTERN = "projects/{}/locations/{}"


def resource(project: str, location: str) -> str:
    return _RESOURCE_PATTERN.format(
        project,
        location,
    )
