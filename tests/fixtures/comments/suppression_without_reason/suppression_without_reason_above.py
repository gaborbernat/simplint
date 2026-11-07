import os

# the plugin under test expects a str, so the wrong type is deliberate
VALUE = 123  # ty: ignore[invalid-argument-type]
# first line of the reason
# second line of the reason
OTHER = os.sep  # type: ignore[misc]
# ruff: ignore[unused-import]
THIRD = 2  # ty: ignore[misc]
#
FOURTH = 3  # type: ignore[misc]

# a reason separated from its line by a blank line

FIFTH = 4  # type: ignore[misc]
# the reason sits above a line of code
SIXTH = 5  # type: ignore[misc]
SEVENTH = 6  # type: ignore[misc]
# a reason on its own line applies to no line of code
# ty: ignore[misc]
EIGHTH = 7
