from functools import wraps


def logged(func):
    def wrapper(*args, **kwargs):
        return func(*args, **kwargs)

    return wrapper


def kept(func):
    @wraps(func)
    def wrapper(*args, **kwargs):
        return func(*args, **kwargs)

    return wrapper
