__all__ = []


class Stream:
    async def __init__(self):
        self.items = []

    async def __len__(self):
        return len(self.items)

    async def __iter__(self):
        for item in self.items:
            yield item

    async def __aiter__(self):
        return self

    async def __anext__(self):
        return 1

    async def __aenter__(self):
        return self

    async def __aexit__(self, *exc_info):
        return None

    async def __call__(self):
        return 1


class Generated:
    async def __aiter__(self):
        yield 1


async def __len__():
    return 0
