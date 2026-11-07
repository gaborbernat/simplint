from dataclasses import dataclass
from typing import Final

DC_KW = {"frozen": True, "slots": True}
SLOTS_KW = {"slots": True}
FROZEN_OFF = {"frozen": False}
MUTATED_KW = {"slots": True}
MUTATED_KW["frozen"] = True
UPDATED_KW = {"slots": True}
UPDATED_KW.update(frozen=True)
FINAL_KW: Final = {"frozen": True}
CALL_KW = dict(frozen=True)
NESTED_KW = {**DC_KW, "slots": True}
REPEATED_KW = {"frozen": True, "frozen": False}


def get_kw() -> dict[str, bool]:
    return {"frozen": True}


def make(**kw: bool) -> type:
    @dataclass(**kw)
    class Passed:
        splat_marker: int

    return Passed


@dataclass(**DC_KW)
class Settled:
    splat_marker: int


@dataclass(**SLOTS_KW)
class SlotsOnly:
    splat_marker: int


@dataclass(**get_kw())
class Computed:
    splat_marker: int


@dataclass(**FROZEN_OFF)
class FrozenOff:
    splat_marker: int


@dataclass(**MUTATED_KW)
class Mutated:
    splat_marker: int


@dataclass(**FINAL_KW)
class FinalKeywords:
    splat_marker: int


@dataclass(**UPDATED_KW)
class Updated:
    splat_marker: int


@dataclass(**CALL_KW)
class CallSet:
    splat_marker: int


@dataclass(**NESTED_KW)
class Nested:
    splat_marker: int


@dataclass(**REPEATED_KW)
class Repeated:
    splat_marker: int
