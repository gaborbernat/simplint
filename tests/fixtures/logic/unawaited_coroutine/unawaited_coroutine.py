import asyncio
import contextlib

from unawaited_coroutine_lib import Service, fetch, publish


async def save(value):
    await asyncio.sleep(value)


async def ticks():
    yield 1


@contextlib.asynccontextmanager
async def session():
    yield


class Store:
    async def flush(self):
        await asyncio.sleep(0)

    @staticmethod
    async def close():
        await asyncio.sleep(0)

    async def run(self):
        save(1)
        self.flush()
        self.close()
        publish("topic")
        await save(2)
        ticks()
        session()
        fetch("url")
        Service()

        class Inner:
            save(3)

        lambda: save(4)

    def sync(self):
        self.flush()


def main():
    save(5)
    asyncio.run(save(6))
    other = Store()
    other.flush()


save(7)
