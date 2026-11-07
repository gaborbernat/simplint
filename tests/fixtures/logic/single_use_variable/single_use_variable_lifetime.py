def produce(rows: list[int]):
    try:
        yield rows[0]
        yield rows[1]
    finally:
        print("closed")


def first_from_next(rows: list[int]) -> None:
    stream = produce(rows)
    first = next(stream)
    print(first)


def first_from_default(rows: list[int]) -> None:
    stream = produce(rows)
    first = next(stream, None)
    print(first)


def reply_from_send(rows: list[int]) -> None:
    stream = produce(rows)
    reply = stream.send(None)
    print(reply)


def stop_from_close(rows: list[int]) -> None:
    stream = produce(rows)
    stream.close()
    print(rows)


async def first_from_anext(source: AsyncSource) -> None:
    stream = source.items()
    first = await anext(stream)
    print(first)


def collected(rows: list[int]) -> None:
    stream = produce(rows)
    items = list(stream)
    print(items)
