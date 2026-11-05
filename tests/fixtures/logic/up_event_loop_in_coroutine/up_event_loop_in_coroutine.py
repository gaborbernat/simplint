import asyncio
from asyncio import get_event_loop


async def run():
    loop = asyncio.get_event_loop()
    return loop


async def nested_sync():
    def later():
        return asyncio.get_event_loop()

    return later


async def with_lambda():
    return lambda: asyncio.get_event_loop()


async def named():
    return get_event_loop()


async def in_class_body():
    class Inner:
        loop = asyncio.get_event_loop()

    return Inner


def plain():
    return asyncio.get_event_loop()


class Holder:
    loop = asyncio.get_event_loop()


async def other_object(client):
    return client.get_event_loop()
