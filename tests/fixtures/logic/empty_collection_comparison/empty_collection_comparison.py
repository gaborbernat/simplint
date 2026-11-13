def check(items, mapping, pair, value, flag):
    if items == []:
        return 1
    elif mapping != {}:
        return 2
    while pair == ():
        pair = (1,)
    label = "empty" if items + [] == [] else "full"
    flags = [items == [], {} == mapping]
    either = flag or items == []
    typed = value == "" or value == [] or value is None
    reversed_check = items == [] or None is items
    assert items == []
    return label, flags, either, typed, reversed_check


def literal_both_sides():
    return [] == []


def string(text):
    return text == ""


def tests(items, flag):
    if not items == []:
        return [item for item in items if item != []]
    if flag and items != ():
        return bool(items == {})
    match flag:
        case True if items == []:
            return None
    return not items == []
