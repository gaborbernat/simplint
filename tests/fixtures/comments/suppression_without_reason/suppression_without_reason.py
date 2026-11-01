import os  # noqa: F401
import sys  # noqa: F401  # re-exported for plugins
VALUE = os.sep  # type: ignore[attr-defined]
OTHER = sys.argv  # type: ignore[attr-defined]  # stub is wrong
THIRD = 1  # ty: ignore
FOURTH = 2  # pyright: ignore[reportGeneralTypeIssues] https://github.com/x/y/issues/1
FIFTH = 3  # pragma: no cover
SIXTH = 4  # pragma: no cover - platform specific
SEVENTH = 5  # noqa: E501 the line is a URL
# a plain comment
EIGHTH = 6  # noqa
NINTH = 7  # noqa: unused because plugins import it
TENTH = 8  # type: ignore
NOTE = 9  # noqa-ish wording is prose
RUFF_BARE = 9  # ruff:ignore[unused-import]
RUFF_REASON = 10  # ruff:ignore[unused-import] # re-exported for plugins
RUFF_SPACED = 11  # ruff: ignore[unused-import]
RUFF_FILE = 12  # ruff:file-ignore[unused-import]
RUFF_BLANKET = 13  # ruff:ignore
PYREFLY = 14  # pyrefly: ignore[bad-assignment]
TIGHT = 15  # type:ignore[misc]
SIMPLINT_BARE = 16  # simplint: ignore[SMP010]
SIMPLINT_REASON = 17  # simplint:ignore[SMP010] names the paper's variable
