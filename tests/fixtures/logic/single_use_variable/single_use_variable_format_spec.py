class Spec:
    def __format__(self, spec: str) -> str:
        return spec


def render(v: Spec) -> str:
    inner = "a{b}"
    return f"{v:{inner}}"


def render_plain(v: Spec) -> str:
    inner = "abc"
    return f"{v:{inner}}"
