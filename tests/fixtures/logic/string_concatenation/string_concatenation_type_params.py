s = "x"


def f[s]() -> str:
    return "a" + s


class C[s]:
    def label(self) -> str:
        return "b" + s
