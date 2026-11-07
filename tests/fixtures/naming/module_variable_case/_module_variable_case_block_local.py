import sys

if sys.version_info < (3, 10):
    msg = "requires Python 3.10"
    raise RuntimeError(msg)

if sys.platform == "linux":
    shared = 1
print(shared)
