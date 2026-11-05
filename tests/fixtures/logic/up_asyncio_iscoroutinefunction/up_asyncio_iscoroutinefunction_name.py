from asyncio import iscoroutinefunction, sleep


def check(task):
    return iscoroutinefunction(task)


async def pause():
    await sleep(0)
