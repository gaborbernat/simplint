from datetime.datetime import strptime


def parse_day(text):
    return strptime(text, "%Y-%m-%d").date()
