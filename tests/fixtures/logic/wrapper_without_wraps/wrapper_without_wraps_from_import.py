from functools import update_wrapper, wraps


def logged(func):
    def wrapper(*args, **kwargs):
        return func(*args, **kwargs)

    return wrapper


def kept(func):
    @wraps(func)
    def wrapper(*args, **kwargs):
        return func(*args, **kwargs)

    return wrapper


def updated(func):
    def wrapper(*args):
        return func(*args)

    update_wrapper(wrapper, func)
    return wrapper


def shadowed(func, wraps):
    def wrapper(*args, **kwargs):
        return func(*args, **kwargs)

    print(wraps)
    return wrapper
