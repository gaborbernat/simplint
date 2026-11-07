import sys

if sys.version_info < (3, 10):
    msg = "requires Python 3.10"
    raise RuntimeError(msg)

with open(__file__) as handle:
    text = handle.read()
    count = len(text)
    print(count)

if sys.platform == "linux":
    shared = 1
print(shared)

if sys.platform == "darwin":
    leaked = 2
print(leaked)
