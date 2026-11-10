from typing import TYPE_CHECKING

if TYPE_CHECKING:
    lazy import json
    from os import PathLike


def dump(value: "PathLike") -> str:
    return json.dumps(value)
