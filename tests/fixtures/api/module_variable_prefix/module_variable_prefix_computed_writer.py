import sys

import _module_variable_prefix_computed_setattr as target


def write(value: int) -> None:
    setattr(target, sys.argv[1], value)
