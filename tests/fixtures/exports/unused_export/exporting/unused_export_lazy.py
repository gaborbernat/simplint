def __getattr__(name: str) -> int:
    return len(name)


__all__ = ["served"]
