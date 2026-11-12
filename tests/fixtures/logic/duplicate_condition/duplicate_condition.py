def label(kind, ready):
    if kind == "a":
        return 1
    elif ready:
        return 2
    elif kind == "a":
        return 3
    return 0


def same_body(kind):
    if kind == "a":
        return 1
    elif kind == "a":
        return 1
    return 0


def apart(kind, ready):
    if kind == "a":
        return 1
    elif ready:
        return 2
    elif kind == "a":
        return 1
    return 0


def calls(stream):
    if stream.ready():
        return 1
    elif stream.ready():
        return 2
    return 0


def operands(first, second):
    return first.ready and second.ready and first.ready, first or first


def effects(stream):
    return stream.readline() or stream.readline()


def walrus(values):
    return (count := len(values)) and count and count


def patterns(value, strict):
    match value:
        case 1:
            return "one"
        case [first, *_]:
            return first
        case 1:
            return "uno"
        case 2 if strict:
            return "strict two"
        case 2:
            return "two"
        case 3 if strict:
            return "strict three"
        case 3 if strict:
            return "three"
        case 4 if value.check():
            return "checked four"
        case 4 if value.check():
            return "four"
    return "many"
