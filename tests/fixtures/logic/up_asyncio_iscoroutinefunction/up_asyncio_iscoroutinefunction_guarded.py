try:
    import asyncio
except ImportError:
    pass


def check(task):
    return asyncio.iscoroutinefunction(task)


async def pause():
    await asyncio.sleep(0)
