from __future__ import annotations

import dataclasses
from typing import Any


class Forms:
    import os
    from pathlib import Path as Route

    tuple_a, (tuple_b, *star_c) = 1, (2, [3])
    [list_d, list_e] = 4, 5
    annotated: int
    alias_d: Any = None
    counter = 0
    counter += 1
    values[0] = 1
    print("class body")

    def method(self, value: int, /, *args: Any, **kwargs: Any) -> Any:
        return value

    class Inner:
        pass


@dataclasses.dataclass
class Point:
    x: int
    y: float = 0.0
