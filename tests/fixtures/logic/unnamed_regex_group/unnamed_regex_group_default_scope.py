import re

m = re.match(r"(a)(b)", "ab")


def first(m=m.group(1)):
    return m


@m.group(1)
def decorated(value: int = 0) -> int:
    return value


def typed[T](value: T) -> T:
    return value
