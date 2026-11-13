import os


def nested_function_binds_its_own(values: list[int]) -> object:
    total = sum(values)
    print(total)

    def inner(items: list[int]) -> int:
        total = len(items)
        return total * 2

    return inner


def lambda_parameter_shadows(values: list[int]) -> object:
    total = sum(values)
    print(total)
    return lambda total: total * 2


def comprehension_target_shadows(values: list[str]) -> list[str]:
    name = values[0]
    print(name)
    return [name for name in values]


def comprehension_iterable_reads_outer(values: list[list[str]]) -> list[str]:
    name = values[0]
    return [name for name in name]


def nested_import_binds_its_own(values: list[str]) -> object:
    path = values[0]
    print(path)

    def inner() -> str:
        from os import path

        return path.sep

    return inner


def nested_function_reads_outer(values: list[int]) -> object:
    total = sum(values)
    print(total)

    def inner() -> int:
        return total

    return inner


def nested_function_rebinds_outer(values: list[int]) -> object:
    total = sum(values)
    print(total)

    def inner() -> None:
        nonlocal total
        total = 0

    return inner


def nested_function_reads_before_binding(values: list[int]) -> object:
    total = sum(values)
    print(total)

    def inner(items: list[int]) -> int:
        print(total)
        total = len(items)
        return total

    return inner


print(os.sep)
