_HIDDEN = 1
SHOWN = 2
_TWICE = 1
_TWICE = 2
_UNUSED = 3
_MODULE_LEVEL = 4
_SEMI = 5; _OTHER = 6
_SUM = 1 + 2
_PAIR = (1, 2)
_DOC = """first
second"""
_COLLIDE = 7
PAIR_A, PAIR_B = 1, 2
LEVEL = _MODULE_LEVEL


def use(collide: int) -> tuple[object, ...]:
    return (_HIDDEN, SHOWN, _TWICE, _SEMI, _OTHER, _SUM, _PAIR, _DOC, _DOC, _COLLIDE, _COLLIDE, collide)


__all__ = ["LEVEL", "PAIR_A", "PAIR_B", "use"]
