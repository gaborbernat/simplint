from dataclasses import dataclass

SLOTS_OFF = {"slots": False}


@dataclass(**SLOTS_OFF)
class SplatSlotsOff:
    splat_marker: int


@dataclass(slots=False)
class KeywordSlotsOff:
    splat_marker: int
