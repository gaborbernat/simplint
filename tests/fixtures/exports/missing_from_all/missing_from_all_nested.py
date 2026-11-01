import sys

if sys.platform == "win32":
    __all__ += ["run"]
else:
    __all__: list[str] = ["run"]


def run_dynamic() -> int:
    return 1
