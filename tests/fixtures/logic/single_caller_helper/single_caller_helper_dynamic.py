__all__ = sorted(["load"])


def load(text: str) -> str:
    return _dynamic(text)


def _dynamic(text: str) -> str:
    return text.strip()
