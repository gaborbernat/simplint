from dataclasses import dataclass


@dataclass(
    eq=False,  # identity semantics on purpose
    slots=True,
)
class Identity:
    splat_marker: int
