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
