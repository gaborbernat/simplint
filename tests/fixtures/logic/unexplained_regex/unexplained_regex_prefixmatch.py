import re


def parse_date(text):
    return re.prefixmatch(r"^(\d{4})-(\d{2})-(\d{2})(?:T[0-2]\d:[0-5]\d)?$", text)
