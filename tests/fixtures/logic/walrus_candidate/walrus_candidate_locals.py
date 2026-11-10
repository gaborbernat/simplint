def locals_read(text: str) -> str:
    found = text.strip()
    if found:
        return str(locals())
    return ""


def vars_read(text: str) -> str:
    found = text.strip()
    if found:
        return str(vars())
    return ""


def eval_read(text: str) -> str:
    found = text.strip()
    if found:
        return str(eval("found"))
    return ""


def exec_read(text: str) -> str:
    found = text.strip()
    if found:
        exec("print(found)")
    return text


def plain(text: str) -> str:
    found = text.strip()
    if found:
        return found
    return ""
