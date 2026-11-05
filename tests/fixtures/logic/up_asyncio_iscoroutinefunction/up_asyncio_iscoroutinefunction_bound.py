import asyncio
from tools import inspect


def check(task):
    return asyncio.iscoroutinefunction(task) or inspect(task)
