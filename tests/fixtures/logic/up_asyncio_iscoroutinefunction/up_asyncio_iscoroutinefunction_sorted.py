import asyncio
import sys


def check(task):
    return asyncio.iscoroutinefunction(task) and sys.version_info > (3, 10)


async def pause():
    await asyncio.sleep(0)
