LIMIT = 1
RATE = 2
DEPTH = 0
GONE = 3


def raise_limit():
    globals()["LIMIT"] = 2


def read_rate():
    return globals()["RATE"]


def deepen():
    globals()["DEPTH"] += 1


def clear():
    del globals()["GONE"]
