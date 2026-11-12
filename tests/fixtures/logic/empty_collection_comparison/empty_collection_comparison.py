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
    assert items == []
    return label, flags, either, typed


def literal_both_sides():
    return [] == []


def string(text):
    return text == ""
