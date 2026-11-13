# A blanket suppression names no code, so it hides every error on its line.
VALUE = 1  # ty: ignore
OTHER = 2  # pyright: ignore
THIRD = 3  # pyrefly: ignore
FOURTH = 4  # mypy: ignore-errors
FIFTH = 5  # TY : IGNORE
SIXTH = 6  # ty: ignore because the stub is wrong
SEVENTH = 7  # ty:ignore[unresolved-import]
EIGHTH = 8  # pyright: ignore[reportAttributeAccessIssue]
NINTH = 9  # ty: ignore[]
TENTH = 10  # ty: ignore[unresolved-import] # pyright: ignore
ELEVENTH = 11  # ty: ignored later
TWELFTH = 12  # type: ignore
THIRTEENTH = "# ty: ignore"
FIFTEENTH = 15  # simplint: ignore
SEVENTEENTH = 17  # noqa
# ruff: file-ignore
# ruff: file-ignore [E501]
EIGHTEENTH = 18  # noqa:
NINETEENTH = 19  # noqa:E501F401
NINETEENTH_SPACED = 19  # ruff: ignore [E501]
NINETEENTH_SPACED_EMPTY = 19  # ruff: ignore  [ ]
TWENTIETH = 20  # ruff: noqa


def twenty_first() -> None:
    # ruff: file-ignore
    pass


# Ruff's PGH003 and PGH004 report the forms below, so SMP004 leaves them out.
TWENTY_SECOND = 22  # noqa
TWENTY_THIRD = 23  # NOQA because plugins import it
TWENTY_FOURTH = 24  # noqa F401
TWENTY_FIFTH = 25  # type: ignore
TWENTY_SIXTH = 26  # pyright:ignore
TWENTY_SEVENTH = 27  # foo # type: ignore
TWENTY_EIGHTH = 28  # ty: ignore # noqa


def twenty_ninth() -> None:
    # ruff: noqa
    # flake8 : NOQA
    pass


# A reason after `type: ignore` leaves PGH003 silent, so SMP004 still reports it.
THIRTIETH = 30  # type: ignore because the stub is wrong
TWENTY_NINTH = 29  # ruffian noqa
