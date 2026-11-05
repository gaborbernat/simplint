from datetime import date, datetime


def parse_day(text):
    return datetime.strptime(text, "%Y-%m-%d").date()
