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
