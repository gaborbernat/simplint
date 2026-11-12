def flip(first, second):
    first = second
    second = first
    return first, second


def flip_attributes(node):
    node.left = node.right
    node.right = node.left


def commented(first, second):
    first = second
    # keep the old value
    second = first
    return first, second


def grow(total, delta, rate, other):
    total += total + delta
    total -= total - delta - rate
    total += total * rate
    total += other + total
    total += delta
    return total


def same(value):
    value = value
    value = value
    return value


def calls(source):
    first = source.read()
    source.read = first
    return first
