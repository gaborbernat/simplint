from dataclasses import dataclass
from typing import ClassVar


@dataclass
class Config:
    tag: "ClassVar[str]" = ""
    name: str = ""
