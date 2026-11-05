from datetime import datetime


def parse_day(text):
    return datetime.strptime(text, "%Y-%m-%d").date()


def parse_both(text):
    return datetime.strptime(text, "%H:%M").time()
