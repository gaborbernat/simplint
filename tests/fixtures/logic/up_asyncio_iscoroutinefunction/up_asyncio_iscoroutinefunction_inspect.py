import asyncio
import inspect


def check(task):
    return asyncio.iscoroutinefunction(task) or inspect.isfunction(task)


async def pause():
    await asyncio.sleep(0)
