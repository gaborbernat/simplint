def quoted_raw(suffix):
    count = 1
    print(rf"{"k" * count=}", suffix)


def quoted(suffix):
    count = 1
    print(f"{"k" * count=}", suffix)


def plain_raw(suffix):
    count = 1
    print(rf"{count=}", suffix)
