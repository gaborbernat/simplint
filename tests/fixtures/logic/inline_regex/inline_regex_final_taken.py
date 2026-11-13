import re

Final = "taken"


def version(text):
    return re.match(r"v\d+", text)
