def load():
    try:
        import shlex
    except ImportError:
        shlex = None
    return shlex


def unused():
    try:
        import shlex
    except ImportError:
        pass
