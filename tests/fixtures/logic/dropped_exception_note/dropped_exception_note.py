__all__ = []


def handled(path):
    try:
        return open(path).read()
    except OSError as error:
        error.add_note(f"while reading {path}")


def raised(path):
    try:
        return open(path).read()
    except OSError as error:
        error.add_note(f"while reading {path}")
        raise


def built(path):
    error = ValueError(path)
    error.add_note("built here")


def built_and_raised(path):
    error = ValueError(path)
    error.add_note("built here")
    raise error


def stored(path, errors):
    try:
        return open(path).read()
    except OSError as error:
        error.add_note(path)
        errors.append(error)


def chained(path):
    try:
        return open(path).read()
    except OSError as error:
        error.add_note(path)
        raise RuntimeError(path) from error


class Holder:
    error = ValueError("class body")
    error.add_note("kept on the class")


def declared(path):
    global shared
    shared = ValueError(path)
    shared.add_note("read elsewhere")


def yielded(path):
    error = ValueError(path)
    error.add_note("built here")
    yield error


def handler_declares(path):
    try:
        return open(path).read()
    except OSError as error:
        global last_path
        last_path = path
        error.add_note(path)


def handler_declares_nonlocal(path):
    last = None

    def read():
        try:
            return open(path).read()
        except OSError as error:
            nonlocal last
            last = path
            error.add_note(path)

    return read(), last
