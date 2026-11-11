import asyncio


def check(inspect, func):
    return asyncio.iscoroutinefunction(func)


def plain(func):
    return asyncio.iscoroutinefunction(func)
