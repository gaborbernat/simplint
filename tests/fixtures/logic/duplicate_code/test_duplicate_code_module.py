def run(items):
    total = 0
    for item in items:
        total += item
    count = len(items)
    log(total)
    log(count)
    return total
