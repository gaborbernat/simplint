def check() -> list[str]:
    return [globals()[name] for name in __all__]


__all__ = ["check"]


RESULT = check()
