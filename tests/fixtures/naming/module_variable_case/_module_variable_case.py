import os

limit = 3
_timeout = 1
PUBLIC_VALUE = 2
logger = os.getenv("X")
Alias = int
pytestmark = []


def use() -> int:
    return limit + _timeout + PUBLIC_VALUE


__all__ = ["PUBLIC_VALUE", "logger"]
for item in range(2):
    pass
with open(__file__) as handle:
    pass
try:
    pass
except OSError as error:
    pass
if (walrus_value := len(__name__)):
    pass

TYPE_CHECKING = False
zip = zip
HANDLER_WALRUS = lambda: (inner := 1)
