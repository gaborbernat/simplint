NAME = "block"
WIDTH = 10
HEIGHT = 20
DEPTH = 30
COLOR = "red"


def build(items):
    total = 0
    for item in items:
        total += item
    count = len(items)
    area = WIDTH * HEIGHT
    volume = area * DEPTH
    log(total, count)
    log(area, volume)
    return total, count, area, volume
