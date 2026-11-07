from . import findings


def write(findings: object) -> object:
    return findings.DbWriter()


def count(items: list[int]) -> int:
    findings = len(items)
    return findings.bit_length()


def each(batches: list[object]) -> None:
    for findings in batches:
        print(findings.DbWriter)


def lengths(words: list[str]) -> list[int]:
    return [findings.count("a") for findings in words]


def writer() -> object:
    return findings.DbWriter()
