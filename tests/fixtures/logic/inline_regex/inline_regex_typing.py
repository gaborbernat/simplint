import re
from typing import Final

_PREFIX: Final = "v"


def version(text: str) -> str | None:
    found = re.match(r"v(\d+)", text)
    return found.group(1) if found else None
