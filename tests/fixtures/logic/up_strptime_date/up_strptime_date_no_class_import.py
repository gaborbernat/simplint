import datetime as dt


def parse_day(text):
    return dt.datetime.strptime(text, "%Y-%m-%d").date()


def parse_time(text):
    return dt.datetime.strptime(text, "%H:%M").time()


def from_function(text):
    from datetime import datetime

    return datetime.strptime(text, "%Y").date()
