import time
from time import time as now


def timed(work):
    start = time.time()
    work()
    return time.time() - start


def both_names(work):
    start = time.time()
    work()
    end = time.time()
    return end - start


def logged(work):
    start = time.time()
    work()
    print("started at", start)
    return time.time() - start


def imported(work):
    started = now()
    work()
    return now() - started


def timestamp():
    return time.time() - 3600


def rebound(work):
    start = time.time()
    start = time.time()
    work()
    return time.time() - start


def since(record):
    start = time.time()
    return record.finished - start
