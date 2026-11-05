from datetime import datetime


def parse_day(text):
    date = datetime.strptime(text, "%Y-%m-%d").date()
    return date
