import asyncio

CHECK = lambda inspect: asyncio.iscoroutinefunction(inspect)
FLAGS = [asyncio.iscoroutinefunction(inspect) for inspect in (print,)]


def helper_shadow(task):
    def helper(inspect):
        return inspect

    return asyncio.iscoroutinefunction(task)
