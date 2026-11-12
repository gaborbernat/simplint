import math

type Nested = list[list[list[list[list[list[list[list[list[list[int]]]]]]]]]]

NESTED: list[list[list[list[list[list[list[list[list[list[int]]]]]]]]]] = []

CALLS = abs(round(float(int(str(len(repr(sorted(set(NESTED)))))))))

KEPT = abs(round(float(int(str(len(repr(sorted(NESTED))))))))

SUM = 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12

DISPLAY = [[[[[[[[[(math.pi, {"e": math.e})]]]]]]]]]


def score(values: list[int], limit: int) -> int:
    if -abs(round(float(int(str(len(repr(sorted(values)))))))) > limit:
        return 0
    elif not (lambda: max(min(sum(values) - 1, 2) * 3, 4) // 5 > 6)():
        return 1
    elif values[0] and values[1] and values[2] and values[3] and values[4] and abs(round(float(int(str(len(repr(sorted(values)))))))):
        return 2
    while (total := sum(value**2 for value in values if value % 2 == 0)) > limit * (limit - 1) // 2 or -(-limit):
        limit += total
    return limit
