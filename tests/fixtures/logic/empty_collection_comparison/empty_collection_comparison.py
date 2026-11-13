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


def compared_with_a_name(items, fallback):
    return items == [] or items == fallback


def missing_key_is_not_empty(data: dict[str, object]) -> bool:
    return data.get("dependencies") != []


def defaulted_key_is_reported(data: dict[str, list[str]]) -> bool:
    return data.get("dependencies", []) != []


def none_default_is_not_empty(data: dict[str, object], item: object) -> bool:
    return data.get("dependencies", None) != [] and getattr(item, "__args__", None) == ()


def getattr_default_is_reported(item: object) -> bool:
    return getattr(item, "__args__", ()) == () or getattr(item, "__args__") == () or vars(item) == {}
