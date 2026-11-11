def check():
    return list(globals()["__all__"])


__all__ = ["check"]

RESULT = check()
