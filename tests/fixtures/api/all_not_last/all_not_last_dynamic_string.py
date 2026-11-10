import sys

__all__ = ["main"]


def main() -> None:
    pass


print(getattr(sys.modules[__name__], "__all__"))
