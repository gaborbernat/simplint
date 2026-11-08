import typing

if typing.TYPE_CHECKING:
    log: typing.Callable[[str], None]
    verbosity: int
    item: int
    limit = 3
else:

    def __getattr__(name: str) -> object: ...


for item in range(3):
    print(item)
VERBOSITY: typing.Final[int] = 0
_verbosity = VERBOSITY
