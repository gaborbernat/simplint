import sys

import single_user_constant_computed as target


def write(value: int) -> None:
    setattr(target, sys.argv[1], value)
