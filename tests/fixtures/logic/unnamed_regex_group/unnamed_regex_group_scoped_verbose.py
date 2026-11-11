import re


def scoped_comment(text):
    return re.search(r"""(?x:
        (\d+)   # the major part (digits
        \.
    )""", text)


def scoped_off(text):
    return re.match(r"(?-x:(a) # (b))", text, re.VERBOSE)
