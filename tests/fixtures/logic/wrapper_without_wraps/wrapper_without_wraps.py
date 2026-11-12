import functools


def logged(func):
    def wrapper(*args, **kwargs):
        return func(*args, **kwargs)

    return wrapper


def method_cache(func):
    def wrapper(self, *args):
        return func(self, *args)

    return wrapper


def keyword_only(func):
    def wrapper(**kwargs):
        return func(**kwargs)

    return wrapper


def wrapped(func):
    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        return func(*args, **kwargs)

    return wrapper


def updated(func):
    def wrapper(*args):
        return func(*args)

    functools.update_wrapper(wrapper, func)
    return wrapper


def renamed(func):
    def wrapper(*args):
        return func(*args)

    wrapper.__name__ = func.__name__
    return wrapper


def factory(callback):
    def run():
        callback()

    return run


def adapter(func):
    def wrapper(*args):
        return func(len(args), *args)

    return wrapper


def reshaped(func):
    def wrapper(*args):
        return func(*[arg * 2 for arg in args])

    return wrapper


def kept(func):
    def wrapper(*args):
        return func(*args)

    return func


def unused(func):
    def wrapper(*args):
        return print(*args)

    return wrapper


def retry(times):
    def decorate(func):
        async def inner(*args, **kwargs):
            for _ in range(times):
                return await func(*args, **kwargs)

        return inner

    return decorate


def no_parameters():
    def wrapper(*args):
        return print(*args)

    return wrapper


def one_line(func):
    def wrapper(*args): return func(*args)
    return wrapper
