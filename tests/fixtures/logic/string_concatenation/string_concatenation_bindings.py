from datetime import *

import contextlib

first, (second, third) = "a", ("b", "c")
items = [1]
items[0] += 1


class Holder:
    pass


holder = Holder()
for holder.attr in [1]:
    pass
with contextlib.nullcontext(1) as holder.attr:
    pass


def collect(*parts, **options):
    return parts[0] + "!" + second + third
