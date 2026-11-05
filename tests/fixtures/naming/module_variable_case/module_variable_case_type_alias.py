type vector = list[float]
type Pair = tuple[int, int]
limit = 3


def norm(values: vector) -> float:
    return sum(values) * limit
