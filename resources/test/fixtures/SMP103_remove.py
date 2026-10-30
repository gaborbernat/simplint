def kept() -> int:
    return 1


def dropped() -> int:
    return 2


__all__ = ["kept", "dropped"]
__all__.remove("dropped")
