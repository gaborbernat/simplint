import asyncio


async def publish(topic):
    await asyncio.sleep(0)


def fetch(url):
    return url


def shuffle(first, second, third):
    return first, second, third


def handler():
    return True


class Service:
    def start(self):
        return self
