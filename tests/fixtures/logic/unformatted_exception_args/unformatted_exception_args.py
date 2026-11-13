class ProjectError(Exception):
    pass


def check(value, limit):
    if value < 0:
        raise ValueError("bad %s", value)
    if value < 1:
        raise KeyError("bad %s and %5.2f", value, limit)
    if value < 2:
        raise ValueError("bad {}", value)
    if value < 3:
        raise TypeError("bad {!r} over {:>4}", value, limit)
    if value < 4:
        raise ValueError("bad %s", value, limit)
    if value < 5:
        raise ValueError("bad %(value)s", value)
    if value < 6:
        raise ValueError("bad {value}", value)
    if value < 7:
        raise ValueError("bad " "%s", value)
    if value < 8:
        raise ValueError("100%% done %s", value)
    if value < 9:
        raise ValueError(
            "bad %s",  # the value
            value,
        )


def fine(value, values):
    if value < 0:
        raise ValueError("bad", value)
    if value < 1:
        raise ValueError("100% sure", value)
    if value < 2:
        raise ProjectError("bad %s", value)
    if value < 3:
        raise ValueError(f"bad {value}")
    if value < 4:
        raise ValueError("bad %s", *values)
    if value < 5:
        raise ValueError("bad %s")
    raise ValueError("{{literal}}", value)


def structured(value, details, errors):
    if value < 0:
        raise SyntaxError("bad %s", details)
    if value < 1:
        raise ExceptionGroup("bad {}", errors)
    if value < 2:
        raise UserWarning("bad %s", value)
