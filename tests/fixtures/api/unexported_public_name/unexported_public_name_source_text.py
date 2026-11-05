__all__ = ["exported"]


def exported() -> str:
    return t"{helper}".interpolations[0].expression


def helper() -> int:
    return 1
