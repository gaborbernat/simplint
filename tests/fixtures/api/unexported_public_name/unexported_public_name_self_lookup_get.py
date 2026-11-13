def handle_a() -> int:
    return 1


def dispatch(kind: str) -> int:
    return globals().get("handle_" + kind)()


__all__ = ["dispatch"]
