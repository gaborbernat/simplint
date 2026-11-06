__all__ = ["listed_area", "listed_perimeter", "LISTED_UNITS"]
LISTED_UNITS = "cm"


def listed_area(side):
    return side * side


def listed_perimeter(side):
    return 4 * side


def listed_hidden():
    return 0
