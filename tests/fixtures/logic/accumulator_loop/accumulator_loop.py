def join(parts):
    text = ""
    for part in parts:
        text += part
    return text


def join_mapped(parts):
    text = ""
    for part in parts:
        text += part.upper()
    return text


def join_bytes(chunks):
    data = b""
    for chunk in chunks:
        data += chunk
    return data


def count(parts):
    total = 0
    for part in parts:
        if part:
            total += 1
    return total


def total_of(pairs):
    result = 0
    for key, value in pairs:
        result += value * 2
    return result


def ternary_iterable(parts, flag):
    size = 0
    for part in parts if flag else []:
        size += len(part)
    return size


def loop_variable_read_after(parts):
    text = ""
    for part in parts:
        text += part
    return text, part


def two_statements(parts):
    text = ""
    for part in parts:
        text += part
        print(part)
    return text


def gap(parts):
    text = ""
    print(text)
    for part in parts:
        text += part
    return text


def reads_itself(parts):
    text = ""
    for part in parts:
        text += text[-1:] + part
    return text


def non_empty_start(parts):
    text = "> "
    for part in parts:
        text += part
    return text


def with_else(parts):
    total = 0
    for part in parts:
        total += part
    else:
        total += 1
    return total


def yielded_parts(parts):
    text = ""
    for part in parts:
        text += yield part
    return text


def tuple_iterable(first, second):
    total = 0
    for value in first, second:
        total += value
    return total


def global_total(parts):
    global grand_total
    grand_total = 0
    for part in parts:
        grand_total += part


def nested_scopes(parts):
    text = ""
    for part in parts:
        text += part

    class Local:
        pass

    def reset():
        nonlocal text
        text = ""

    return text, Local, reset
