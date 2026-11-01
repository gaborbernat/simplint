from typing import Annotated
from typing import NewType, TypeVar

import logging

COUNT = 3
NAME = "simplint"
RATE = 0.5
FLAG = True
DATA = b"x"
LOGGER = logging.getLogger(__name__)
LIMIT: int = 10
ALREADY: "Final[int]" = 1
CHANGED = 1
CHANGED = 2
Alias = int
__version__ = "1.0"
FIRST, SECOND = 1, 2
T = TypeVar("T")
UserId = NewType("UserId", int)
OPTIONAL: int | None = 1
QUALIFIED: Annotated[Final[int], "doc"] = 2
DESCRIBED: Annotated[int, "doc"] = 3

TYPE_CHECKING = False

JSON = dict[str, Any]
MODE = Literal["a", "b"]
_path_t = str | bytes
MAYBE = int | None
FLAGS = READ | WRITE
