def logged(func):
    def wrapper(*args, **kwargs):
        return func(*args, **kwargs)

    return wrapper


def shadowed(func, functools):
    def wrapper(*args, **kwargs):
        return func(*args, **kwargs)

    print(functools)
    return wrapper
