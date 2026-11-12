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
