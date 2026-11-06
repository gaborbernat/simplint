import sys

__all__ = ["conditional_base"]


def conditional_base():
    return 1


if sys.platform == "win32":

    def conditional_extra():
        return 2

    __all__.append("conditional_extra")
