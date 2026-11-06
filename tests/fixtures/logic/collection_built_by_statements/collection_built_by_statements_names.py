def keyed(name: str, value: int) -> dict[str, int]:
    result = {}
    result[name] = value
    result["total"] = 1
    return result


def listed(items: list[str], first: str) -> list[str]:
    names = []
    names.append(first)
    names.append(items[0])
    return names
