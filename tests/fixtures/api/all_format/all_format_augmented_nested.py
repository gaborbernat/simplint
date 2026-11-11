import sys

a = b = c = 1

if sys.platform == "win32":
    __all__ = ("a",)
else:
    __all__ = ("a", "c")
__all__ += ("b",)
