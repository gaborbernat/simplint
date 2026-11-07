__all__ = []

from dataclasses import dataclass

OPTS = {"version": "1.0.0"}
CLASH = {"name": "other"}


def kw_source() -> dict[str, str]:
    return {}


@dataclass(frozen=True, slots=True)
class SplatRelease:
    name: str
    version: str


SplatRelease("simplint", **OPTS)
SplatRelease("simplint", "1.0.0")


@dataclass(frozen=True, slots=True)
class SplatTag:
    name: str
    version: str


SplatTag("simplint", "1.0.0")
SplatTag("simplint", **CLASH)


@dataclass(frozen=True, slots=True)
class SplatPin:
    name: str
    version: str


SplatPin("a", "1.0.0")
SplatPin("a", **kw_source())
