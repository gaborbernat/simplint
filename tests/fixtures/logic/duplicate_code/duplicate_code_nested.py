def outer(flag, items):
    if flag:
        total = 0
        for item in items:
            total += item
        count = len(items)
        log(total)
        log(count)
        return total
    return None
