class Interrupted(Exception):
    pass


class Session:
    Interrupted = Interrupted


def main() -> type:
    return Interrupted
