def quoted(suffix: str) -> str:
    label = 'say "hi"'
    return f"{f'{label}'}" + suffix


def other(value: str, suffix: str) -> str:
    stripped = value.strip()
    return stripped + suffix
