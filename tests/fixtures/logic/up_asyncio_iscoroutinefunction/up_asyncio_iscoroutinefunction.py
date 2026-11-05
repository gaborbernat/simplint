import asyncio


async def run(task):
    await asyncio.sleep(0)
    if asyncio.iscoroutinefunction(task):
        return await task()
    return task()
