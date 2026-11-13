import functools
from functools import cache, cached_property, lru_cache


@functools.cache
def gen(n):
    yield n


@lru_cache(maxsize=None)
def gen2(n):
    yield from range(n)


class A:
    @cache
    def m(self):
        yield 1

    @staticmethod
    @cache
    def s():
        yield 1

    @classmethod
    @lru_cache
    def c(cls):
        yield 1

    @cached_property
    def p(self):
        yield 1


def outer():
    @cache
    def inner():
        yield 1
    return inner


@cache
def values(n):
    return list(range(n))


@functools.lru_cache
async def stream():
    yield 1


@print
def plain():
    yield 1
