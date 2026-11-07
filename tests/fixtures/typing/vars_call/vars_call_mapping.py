from collections.abc import Callable


def bare_return(obj: object) -> dict[str, object]:
    return vars(obj)


def bare_dict_copy(obj: object) -> dict[str, object]:
    return dict(vars(obj))


def items_comprehension(obj: object) -> dict[str, object]:
    return {key: value for key, value in vars(obj).items()}


def filtered_comprehension(obj: object) -> dict[str, object]:
    return {key: value for key, value in vars(obj).items() if value is not None}


def merged_left(obj: object) -> dict[str, object]:
    return vars(obj) | {"prog": "x"}


def merged_right(obj: object) -> dict[str, object]:
    return {"prog": "x"} | vars(obj)


def unpacked(obj: object) -> dict[str, object]:
    return {**vars(obj), "prog": "x"}


def merged_comprehension(obj: object) -> dict[str, object]:
    return {key: value for key, value in vars(obj).items()} | {"prog": "x"}


def list_of_keys(obj: object) -> list[str]:
    return list(vars(obj))


def value_of_mapping(obj: object) -> dict[str, dict[str, object]]:
    return {"attributes": vars(obj)}


def values_comprehension(obj: object) -> list[object]:
    return [value for value in vars(obj).values()]


def items_list(obj: object) -> list[tuple[str, object]]:
    return list(vars(obj).items())


def keys_list_comprehension(obj: object) -> list[str]:
    return [key for key, value in vars(obj).items()]


def keys_from_other_iterable(names: list[str], obj: object) -> dict[str, int]:
    return {name: 1 for name in names if vars(obj).items()}


def shadowed_dict(dict: Callable[[object], dict[str, object]], obj: object) -> dict[str, object]:
    return dict(vars(obj))
