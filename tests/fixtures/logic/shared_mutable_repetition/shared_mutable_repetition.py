range_of = range


def board(size):
    grid = [[0] * size] * size
    grid[0][0] = 1
    return grid


def buckets(size):
    rows = size * [[]]
    rows[1].append(2)
    return rows


def annotated(size):
    grid = [[0] * size] * size
    grid[0][0]: int = 1
    return grid


def extended(size):
    rows = [[]] * size
    rows[0] += [1]
    return rows


def removed(size):
    rows = [{"key": 1}] * size
    del rows[0]["key"]
    return rows


def renamed(size):
    rows = [[], []] * size
    rows[0].attribute = 1
    return rows


def shadowed(size, _):
    rows = [[_]] * size
    rows[0].append(1)
    return rows


def tupled(size):
    rows = ([],) * size
    rows[0].append(1)
    return rows


class Table:
    def __init__(self, size):
        self.cells = [dict()] * size

    def put(self, index, key):
        self.cells[index][key] = 1


def read_only(size):
    return [[0] * size] * size


def stored_not_changed(size):
    grid = [[0] * size] * size
    return grid[0][0]


def immutable(size):
    values = [None] * size
    values[0] = 1
    return values


def single(size):
    rows = [[]] * 1
    rows[0].append(size)
    return rows
