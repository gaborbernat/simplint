from typing import TYPE_CHECKING
from os import path

if TYPE_CHECKING:
    from os import PathLike
else:
    PathLike = str

if TYPE_CHECKING:
    from os import *

if TYPE_CHECKING:
    VALUE = 1
