from __future__ import annotations

from dataclasses import dataclass


@dataclass
class Node:
    value: int
    parent: None | "Node" = None
