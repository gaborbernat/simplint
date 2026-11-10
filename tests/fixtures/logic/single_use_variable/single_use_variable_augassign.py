class Counter:
    def __init__(self) -> None:
        self.total = 0

    def bump(self) -> int:
        self.total += 10
        return 1

    def run(self) -> int:
        step = self.bump()
        self.total += step
        return self.total


def closure() -> int:
    total = 0

    def bump() -> int:
        nonlocal total
        total += 10
        return 1

    step = bump()
    total += step
    return total


def local_call(items: list[int]) -> int:
    count = 0
    step = len(items)
    count += step
    return count


def local_literal(items: list[int]) -> int:
    count = len(items)
    step = 2
    count += step
    return count


def augmented_on_holder() -> None:
    holder = [0]
    holder[0] += 2
